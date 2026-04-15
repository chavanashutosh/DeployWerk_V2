//! GitHub App REST API helpers (JWT for installation access tokens).
//!
//! Webhook deliveries use **HMAC** (`X-Hub-Signature-256`); JWT here is for calling GitHub’s
//! `POST /app/installations/{id}/access_tokens` and related APIs.

use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;

#[derive(Serialize)]
struct AppJwtClaims {
    /// Issued at (Unix seconds).
    iat: i64,
    /// Expiration (Unix seconds); GitHub allows up to 10 minutes.
    exp: i64,
    /// GitHub App numeric id as string.
    iss: String,
}

/// Encode a short-lived RS256 JWT to authenticate as the GitHub App (PEM PKCS#8 or RSA private key).
pub fn encode_github_app_jwt(app_id: u64, pem: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now().timestamp();
    let claims = AppJwtClaims {
        iat: now - 60,
        exp: now + 600,
        iss: app_id.to_string(),
    };
    let key = EncodingKey::from_rsa_pem(pem.as_bytes())?;
    let header = Header::new(Algorithm::RS256);
    encode(&header, &claims, &key)
}
