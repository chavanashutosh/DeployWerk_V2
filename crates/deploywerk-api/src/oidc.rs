//! Authentik (or any OIDC provider) authorization code + PKCE; issues DeployWerk session JWTs.

use std::sync::Arc;

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use deploywerk_core::UserSummary;
use openidconnect::core::{CoreIdTokenClaims, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, IssuerUrl, Nonce, PkceCodeVerifier, RedirectUrl,
    TokenResponse,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::issue_token;
use crate::error::ApiError;
use crate::handlers::AuthResponse;
use crate::AppState;

/// Runtime OIDC client config (from env). Issuer URL must match Authentik application issuer.
pub struct OidcConfigState {
    pub issuer: String,
    /// Stored on `users.idp_issuer` (normalized, no trailing slash).
    pub idp_issuer_db: String,
    pub client_id: String,
    pub client_secret: String,
    /// Registered OAuth redirect URI for the SPA (optional hint for clients).
    pub redirect_uri: Option<String>,
    pub http: reqwest::Client,
    pub metadata: tokio::sync::Mutex<Option<CoreProviderMetadata>>,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/auth/oidc/config", get(oidc_config))
        .route("/api/v1/auth/oidc/callback", post(oidc_callback))
}

#[derive(Serialize)]
struct OidcConfigResponse {
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    issuer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    redirect_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    authorization_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scopes: Option<String>,
}

async fn oidc_config(State(state): State<Arc<AppState>>) -> Result<Json<OidcConfigResponse>, ApiError> {
    let Some(ref oidc) = state.oidc else {
        return Ok(Json(OidcConfigResponse {
            enabled: false,
            issuer: None,
            client_id: None,
            redirect_uri: None,
            authorization_endpoint: None,
            token_endpoint: None,
            scopes: None,
        }));
    };

    let meta = oidc_metadata(oidc).await?;
    let token_ep = meta
        .token_endpoint()
        .map(|u| u.to_string())
        .ok_or_else(|| ApiError::Internal)?;
    Ok(Json(OidcConfigResponse {
        enabled: true,
        issuer: Some(oidc.issuer.clone()),
        client_id: Some(oidc.client_id.clone()),
        redirect_uri: oidc.redirect_uri.clone(),
        authorization_endpoint: Some(meta.authorization_endpoint().to_string()),
        token_endpoint: Some(token_ep),
        scopes: Some("openid profile email".into()),
    }))
}

async fn oidc_metadata(oidc: &OidcConfigState) -> Result<CoreProviderMetadata, ApiError> {
    let mut lock = oidc.metadata.lock().await;
    if let Some(ref m) = *lock {
        return Ok(m.clone());
    }
    let issuer = IssuerUrl::new(oidc.issuer.clone()).map_err(|_| ApiError::Internal)?;
    let discovered = CoreProviderMetadata::discover_async(issuer, &oidc.http)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "OIDC provider metadata discovery failed");
            ApiError::Internal
        })?;
    *lock = Some(discovered.clone());
    Ok(discovered)
}

#[derive(Deserialize)]
struct OidcCallbackBody {
    code: String,
    code_verifier: String,
    redirect_uri: String,
    nonce: String,
}

async fn oidc_callback(
    State(state): State<Arc<AppState>>,
    Json(body): Json<OidcCallbackBody>,
) -> Result<Json<AuthResponse>, ApiError> {
    let Some(ref oidc) = state.oidc else {
        return Err(ApiError::BadRequest("OIDC is not configured".into()));
    };

    let meta = oidc_metadata(oidc).await?;
    let client = openidconnect::core::CoreClient::from_provider_metadata(
        meta,
        ClientId::new(oidc.client_id.clone()),
        Some(ClientSecret::new(oidc.client_secret.clone())),
    )
    .set_redirect_uri(
        RedirectUrl::new(body.redirect_uri.clone()).map_err(|_| ApiError::BadRequest("invalid redirect_uri".into()))?,
    );

    let verifier = PkceCodeVerifier::new(body.code_verifier);
    let token = client
        .exchange_code(AuthorizationCode::new(body.code))
        .map_err(|_| ApiError::Internal)?
        .set_pkce_verifier(verifier)
        .request_async(&oidc.http)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "OIDC token exchange failed");
            ApiError::Unauthorized
        })?;

    let id_token = token.id_token().ok_or(ApiError::Unauthorized)?;
    let nonce = Nonce::new(body.nonce);
    let verifier = client.id_token_verifier();
    let claims = id_token
        .claims(&verifier, &nonce)
        .map_err(|e| {
            tracing::warn!(error = %e, "id_token validation failed");
            ApiError::Unauthorized
        })?;

    let user_id = jit_oidc_user(&state.pool, oidc, claims).await?;
    let token_jwt = issue_token(user_id, &state.jwt_secret)?;

    let pref: Option<(Option<Uuid>, Option<Uuid>, serde_json::Value)> = sqlx::query_as(
        "SELECT current_team_id, current_organization_id, COALESCE(settings_json, '{}'::jsonb) FROM user_preferences WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let (current_team_id, current_organization_id, settings) = pref
        .map(|(t, o, s)| (t, o, Some(s)))
        .unwrap_or((None, None, Some(serde_json::json!({}))));

    let row: (String, Option<String>, bool) = sqlx::query_as(
        "SELECT email, name, is_platform_admin FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let user = crate::rbac::user_summary_with_rbac(
        &state.pool,
        UserSummary {
            id: user_id,
            email: row.0,
            name: row.1,
            current_team_id,
            current_organization_id,
            settings,
            is_platform_admin: row.2,
            organization_admin_organization_ids: vec![],
            application_memberships: vec![],
        },
    )
    .await?;

    Ok(Json(AuthResponse {
        token: token_jwt,
        user,
    }))
}

async fn jit_oidc_user(
    pool: &PgPool,
    oidc: &OidcConfigState,
    claims: &CoreIdTokenClaims,
) -> Result<Uuid, ApiError> {
    let sub = claims.subject().to_string();
    let issuer = oidc.idp_issuer_db.clone();
    let email = claims
        .email()
        .map(|e| e.as_str().to_lowercase())
        .or_else(|| {
            claims
                .preferred_username()
                .map(|u| u.as_str().to_lowercase())
        })
        .filter(|e| !e.is_empty() && e.contains('@'))
        .ok_or_else(|| ApiError::BadRequest("OIDC token missing email (claim email or preferred_username)".into()))?;
    let name = claims
        .name()
        .and_then(|n| n.get(None))
        .map(|s| s.as_str().to_string())
        .or_else(|| claims.preferred_username().map(|u| u.as_str().to_string()));

    let by_idp: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM users WHERE idp_issuer = $1 AND idp_subject = $2",
    )
    .bind(&issuer)
    .bind(&sub)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    if let Some(id) = by_idp {
        sqlx::query("UPDATE users SET email = $1, name = COALESCE($2, name) WHERE id = $3")
            .bind(&email)
            .bind(&name)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|_| ApiError::Internal)?;
        return Ok(id);
    }

    let by_email: Option<(Uuid, Option<String>)> = sqlx::query_as(
        "SELECT id, idp_subject FROM users WHERE LOWER(email) = LOWER($1)",
    )
    .bind(&email)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    if let Some((id, existing_sub)) = by_email {
        if existing_sub.is_some() && existing_sub.as_deref() != Some(sub.as_str()) {
            return Err(ApiError::Conflict(
                "email already linked to another SSO identity".into(),
            ));
        }
        sqlx::query(
            "UPDATE users SET idp_issuer = $1, idp_subject = $2, name = COALESCE($3, name) WHERE id = $4",
        )
        .bind(&issuer)
        .bind(&sub)
        .bind(&name)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
        return Ok(id);
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    let mut tx = pool.begin().await.map_err(|_| ApiError::Internal)?;
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, created_at, idp_issuer, idp_subject) VALUES ($1, $2, NULL, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(&email)
    .bind(&name)
    .bind(now)
    .bind(&issuer)
    .bind(&sub)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;

    sqlx::query(
        "INSERT INTO user_preferences (user_id, settings_json) VALUES ($1, '{}'::jsonb) ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;

    tx.commit().await.map_err(|_| ApiError::Internal)?;
    Ok(id)
}
