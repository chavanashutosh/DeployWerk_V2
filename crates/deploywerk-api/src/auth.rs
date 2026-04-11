use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use chrono::{Duration, Utc};
use deploywerk_core::{TeamRole, UserId};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::ApiError;

const JWT_EXPIRY_HOURS: i64 = 72;

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub exp: i64,
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

pub async fn user_id_from_token(
    pool: &SqlitePool,
    token: &str,
    secret: &str,
) -> Result<UserId, ApiError> {
    let data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| ApiError::Unauthorized)?;

    let id = Uuid::parse_str(&data.claims.sub).map_err(|_| ApiError::Unauthorized)?;

    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM users WHERE id = ?")
        .bind(id.to_string())
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    if exists == 0 {
        return Err(ApiError::Unauthorized);
    }

    Ok(id)
}

pub fn role_from_db(s: &str) -> TeamRole {
    TeamRole::parse(s).unwrap_or(TeamRole::Member)
}
