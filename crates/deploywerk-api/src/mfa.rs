//! Minimal TOTP MFA for local-password accounts (Phase 1).

use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use totp_rs::{Algorithm, Secret, TOTP};

use crate::auth::require_principal;
use crate::crypto_keys::{decrypt_private_key, encrypt_private_key};
use crate::error::ApiError;
use crate::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/me/mfa", get(get_mfa_status))
        .route("/api/v1/me/mfa/totp/enroll", post(enroll_totp))
        .route("/api/v1/me/mfa/totp/verify", post(verify_totp))
}

#[derive(Serialize)]
struct MfaStatusResponse {
    totp_enabled: bool,
}

async fn get_mfa_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<MfaStatusResponse>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_read()?;
    let row: Option<(bool,)> =
        sqlx::query_as("SELECT enabled FROM user_totp WHERE user_id = $1")
            .bind(p.user_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
    Ok(Json(MfaStatusResponse {
        totp_enabled: row.map(|r| r.0).unwrap_or(false),
    }))
}

#[derive(Serialize)]
struct EnrollResponse {
    secret_base32: String,
    otpauth_url: String,
}

async fn enroll_totp(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<EnrollResponse>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;

    let email: Option<String> =
        sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
            .bind(p.user_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
    let label = email.unwrap_or_else(|| format!("user-{}", p.user_id.simple()));

    let secret = Secret::generate_secret();
    let secret_base32 = secret.to_encoded().to_string();

    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        Secret::Encoded(secret_base32.clone()).to_bytes().map_err(|_| ApiError::Internal)?,
        Some("DeployWerk".into()),
        label,
    )
    .map_err(|_| ApiError::Internal)?;

    let otpauth_url = totp.get_url();

    let ct = encrypt_private_key(&state.server_key_encryption_key, secret_base32.as_bytes())
        .map_err(|_| ApiError::Internal)?;
    let now = Utc::now();

    sqlx::query(
        r#"INSERT INTO user_totp (user_id, secret_ciphertext, enabled, created_at, updated_at)
           VALUES ($1,$2,FALSE,$3,$3)
           ON CONFLICT (user_id) DO UPDATE SET secret_ciphertext = EXCLUDED.secret_ciphertext, enabled = FALSE, updated_at = EXCLUDED.updated_at"#,
    )
    .bind(p.user_id)
    .bind(ct)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(EnrollResponse {
        secret_base32,
        otpauth_url,
    }))
}

#[derive(Deserialize)]
struct VerifyBody {
    code: String,
}

async fn verify_totp(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<VerifyBody>,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;

    let row: Option<(Vec<u8>,)> =
        sqlx::query_as("SELECT secret_ciphertext FROM user_totp WHERE user_id = $1")
            .bind(p.user_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
    let Some((ct,)) = row else {
        return Err(ApiError::BadRequest("enroll TOTP first".into()));
    };
    let plain = decrypt_private_key(&state.server_key_encryption_key, &ct)
        .map_err(|_| ApiError::Internal)?;
    let secret_base32 = String::from_utf8(plain).map_err(|_| ApiError::Internal)?;

    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        Secret::Encoded(secret_base32).to_bytes().map_err(|_| ApiError::Internal)?,
        None,
        "DeployWerk".into(),
    )
    .map_err(|_| ApiError::Internal)?;

    let ok = totp.check_current(&body.code.trim()).unwrap_or(false);
    if !ok {
        return Err(ApiError::Unauthorized);
    }

    sqlx::query("UPDATE user_totp SET enabled = TRUE, updated_at = $1 WHERE user_id = $2")
        .bind(Utc::now())
        .bind(p.user_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    Ok(StatusCode::NO_CONTENT)
}

