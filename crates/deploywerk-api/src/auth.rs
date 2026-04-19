use std::net::IpAddr;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::http::{header::AUTHORIZATION, HeaderMap};
use chrono::{Duration, Utc};
use ipnetwork::IpNetwork;
use deploywerk_core::{TeamRole, TokenScopes, UserId};
use hex;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use crate::DbPool;
use uuid::Uuid;

use crate::error::ApiError;
use crate::AppState;

const JWT_EXPIRY_HOURS: i64 = 72;

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub exp: i64,
}

#[derive(Debug, Clone)]
pub struct Principal {
    pub user_id: UserId,
    /// Session via login/register JWT (full access). API tokens carry explicit scopes.
    pub is_jwt: bool,
    pub scopes: TokenScopes,
}

impl Principal {
    pub fn require_jwt(&self) -> Result<(), ApiError> {
        if self.is_jwt {
            Ok(())
        } else {
            Err(ApiError::Forbidden)
        }
    }

    pub fn require_read(&self) -> Result<(), ApiError> {
        if self.scopes.read {
            Ok(())
        } else {
            Err(ApiError::Forbidden)
        }
    }

    pub fn require_write(&self) -> Result<(), ApiError> {
        if self.scopes.write {
            Ok(())
        } else {
            Err(ApiError::Forbidden)
        }
    }

    pub fn require_deploy(&self) -> Result<(), ApiError> {
        if self.scopes.deploy {
            Ok(())
        } else {
            Err(ApiError::Forbidden)
        }
    }
}

pub fn hash_api_token_raw(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn generate_api_token_value() -> String {
    format!("dw_{}", Uuid::new_v4().simple())
}

pub fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| ApiError::Internal)?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, password_hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(password_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

pub fn issue_token(user_id: UserId, secret: &str) -> Result<String, ApiError> {
    let exp = Utc::now() + Duration::hours(JWT_EXPIRY_HOURS);
    let claims = JwtClaims {
        sub: user_id.to_string(),
        exp: exp.timestamp(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| ApiError::Internal)
}

pub fn parse_bearer_token(auth_header: Option<&str>) -> Option<&str> {
    let h = auth_header?;
    let prefix = "Bearer ";
    h.strip_prefix(prefix).map(str::trim)
}

/// Best-effort client IP for API-token CIDR checks. Prefer a trusted reverse proxy that sets
/// `X-Forwarded-For` (first hop) or `X-Real-IP`; otherwise returns `None`.
pub fn peer_ip_from_headers(headers: &HeaderMap) -> Option<IpAddr> {
    if let Some(xff) = headers.get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        let first = xff
            .split(',')
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty())?;
        if let Ok(ip) = first.parse::<IpAddr>() {
            return Some(ip);
        }
    }
    if let Some(xr) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        let t = xr.trim();
        if let Ok(ip) = t.parse::<IpAddr>() {
            return Some(ip);
        }
    }
    None
}

fn api_token_ip_allowed(client_ip: Option<IpAddr>, allowed: &Option<serde_json::Value>) -> Result<(), ApiError> {
    let Some(raw) = allowed else {
        return Ok(());
    };
    let Some(arr) = raw.as_array() else {
        return Ok(());
    };
    if arr.is_empty() {
        return Ok(());
    }
    let Some(ip) = client_ip else {
        return Err(ApiError::Forbidden);
    };
    for entry in arr {
        let Some(s) = entry.as_str().map(str::trim).filter(|s| !s.is_empty()) else {
            continue;
        };
        if let Ok(net) = s.parse::<IpNetwork>() {
            if net.contains(ip) {
                return Ok(());
            }
        }
    }
    Err(ApiError::Forbidden)
}

pub async fn resolve_principal(
    pool: &DbPool,
    token: &str,
    secret: &str,
    client_ip: Option<IpAddr>,
) -> Result<Principal, ApiError> {
    if let Ok(data) = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    ) {
        let id = Uuid::parse_str(&data.claims.sub).map_err(|_| ApiError::Unauthorized)?;
        let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM users WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(|_| ApiError::Internal)?;
        if exists == 0 {
            return Err(ApiError::Unauthorized);
        }
        return Ok(Principal {
            user_id: id,
            is_jwt: true,
            scopes: TokenScopes::full(),
        });
    }

    let h = hash_api_token_raw(token);
    let now = Utc::now();
    let row: Option<(Uuid, String, Option<serde_json::Value>)> = sqlx::query_as(
        "SELECT user_id, scopes, allowed_cidrs FROM api_tokens WHERE token_hash = $1 AND (expires_at IS NULL OR expires_at > $2)",
    )
            .bind(&h)
            .bind(now)
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::Internal)?;

    let Some((user_id, scopes_json, allowed_cidrs)) = row else {
        return Err(ApiError::Unauthorized);
    };

    api_token_ip_allowed(client_ip, &allowed_cidrs)?;

    let scopes = TokenScopes::parse_json(&scopes_json);
    if !scopes.read && !scopes.write && !scopes.deploy {
        return Err(ApiError::Forbidden);
    }

    Ok(Principal {
        user_id,
        is_jwt: false,
        scopes,
    })
}

pub fn role_from_db(s: &str) -> TeamRole {
    TeamRole::parse(s).unwrap_or(TeamRole::Member)
}

pub async fn require_principal(state: &AppState, headers: &HeaderMap) -> Result<Principal, ApiError> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    let token = parse_bearer_token(auth).ok_or(ApiError::Unauthorized)?;
    let client_ip = peer_ip_from_headers(headers);
    resolve_principal(&state.pool, token, &state.jwt_secret, client_ip).await
}
