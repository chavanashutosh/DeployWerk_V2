//! Team-scoped secrets; application env values may reference `dw_secret:NAME`.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{delete, get};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::require_principal;
use crate::audit::try_log_team_audit;
use crate::crypto_keys::{decrypt_private_key, encrypt_private_key};
use crate::error::ApiError;
use crate::rbac::{require_team_member, require_team_mutator};
use crate::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/teams/{team_id}/secrets", get(list_secrets).post(upsert_secret))
        .route("/api/v1/teams/{team_id}/secrets/{name}", delete(delete_secret))
        .route(
            "/api/v1/teams/{team_id}/secrets/{name}/versions",
            get(list_secret_versions),
        )
}

#[derive(Serialize)]
struct TeamSecretMeta {
    name: String,
    updated_at: chrono::DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    latest_version: Option<i32>,
}

async fn list_secrets(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamSecretMeta>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let rows: Vec<(String, chrono::DateTime<Utc>)> = sqlx::query_as(
        "SELECT name, updated_at FROM team_secrets WHERE team_id = $1 ORDER BY name",
    )
    .bind(team_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(|(name, updated_at)| TeamSecretMeta {
                name,
                updated_at,
                latest_version: None,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct UpsertSecretBody {
    name: String,
    value: String,
}

async fn upsert_secret(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<UpsertSecretBody>,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let name = body.name.trim();
    if name.is_empty() || name.len() > 128 {
        return Err(ApiError::BadRequest("invalid secret name".into()));
    }

    let ct = encrypt_private_key(&state.server_key_encryption_key, body.value.as_bytes())
        .map_err(|_| ApiError::Internal)?;
    let now = Utc::now();

    let mut tx = state.pool.begin().await.map_err(|_| ApiError::Internal)?;

    let cur_version: Option<i32> = sqlx::query_scalar(
        "SELECT latest_version FROM team_secrets WHERE team_id = $1 AND name = $2",
    )
    .bind(team_id)
    .bind(name)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;

    let next_version = cur_version.unwrap_or(0).saturating_add(1).max(1);

    sqlx::query(
        r#"INSERT INTO team_secret_versions
           (id, team_id, name, version, value_ciphertext, created_at, created_by_user_id)
           VALUES ($1,$2,$3,$4,$5,$6,$7)"#,
    )
    .bind(Uuid::new_v4())
    .bind(team_id)
    .bind(name)
    .bind(next_version)
    .bind(&ct)
    .bind(now)
    .bind(p.user_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;

    sqlx::query(
        r#"INSERT INTO team_secrets (id, team_id, name, value_ciphertext, created_at, updated_at, latest_version)
           VALUES ($1, $2, $3, $4, $5, $5, $6)
           ON CONFLICT (team_id, name) DO UPDATE SET
             value_ciphertext = EXCLUDED.value_ciphertext,
             updated_at = EXCLUDED.updated_at,
             latest_version = EXCLUDED.latest_version"#,
    )
    .bind(Uuid::new_v4())
    .bind(team_id)
    .bind(name)
    .bind(&ct)
    .bind(now)
    .bind(next_version)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;

    tx.commit().await.map_err(|_| ApiError::Internal)?;

    try_log_team_audit(
        &state.pool,
        team_id,
        p.user_id,
        "team_secret.upsert",
        "team_secret",
        None,
        serde_json::json!({ "name": name, "version": next_version }),
        None,
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

async fn delete_secret(
    State(state): State<Arc<AppState>>,
    Path((team_id, name)): Path<(Uuid, String)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let n = sqlx::query("DELETE FROM team_secrets WHERE team_id = $1 AND name = $2")
        .bind(team_id)
        .bind(name.trim())
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?
        .rows_affected();
    if n == 0 {
        return Err(ApiError::NotFound);
    }
    sqlx::query("DELETE FROM team_secret_versions WHERE team_id = $1 AND name = $2")
        .bind(team_id)
        .bind(name.trim())
        .execute(&state.pool)
        .await
        .ok();

    try_log_team_audit(
        &state.pool,
        team_id,
        p.user_id,
        "team_secret.delete",
        "team_secret",
        None,
        serde_json::json!({ "name": name.trim() }),
        None,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
struct SecretVersionRow {
    version: i32,
    created_at: chrono::DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_by_user_id: Option<Uuid>,
}

async fn list_secret_versions(
    State(state): State<Arc<AppState>>,
    Path((team_id, name)): Path<(Uuid, String)>,
    headers: HeaderMap,
) -> Result<Json<Vec<SecretVersionRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let nm = name.trim();
    if nm.is_empty() {
        return Err(ApiError::BadRequest("invalid secret name".into()));
    }

    let rows: Vec<(i32, chrono::DateTime<Utc>, Option<Uuid>)> = sqlx::query_as(
        r#"SELECT version, created_at, created_by_user_id
           FROM team_secret_versions
           WHERE team_id = $1 AND name = $2
           ORDER BY version DESC"#,
    )
    .bind(team_id)
    .bind(nm)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(|(version, created_at, created_by_user_id)| SecretVersionRow {
                version,
                created_at,
                created_by_user_id,
            })
            .collect(),
    ))
}

/// Resolve `dw_secret:NAME` placeholders in env values for deploy.
pub async fn resolve_dw_secret_env_values(
    pool: &sqlx::PgPool,
    team_id: Uuid,
    key: &[u8; 32],
    rows: Vec<(String, String, bool)>,
) -> Result<Vec<(String, String, bool)>, String> {
    let mut out = Vec::with_capacity(rows.len());
    for (k, v, is_secret) in rows {
        let prefix = "dw_secret:";
        let resolved = if v.trim_start().starts_with(prefix) {
            let raw = v.trim().strip_prefix(prefix).unwrap_or("").trim();
            let (name, version_opt) = if let Some((n, ver)) = raw.split_once('@') {
                let n = n.trim();
                let ver = ver.trim();
                let parsed: i32 = ver
                    .parse()
                    .map_err(|_| format!("invalid secret version `{ver}`"))?;
                (n, Some(parsed))
            } else {
                (raw, None)
            };
            if name.is_empty() {
                return Err("empty dw_secret name".into());
            }
            let blob: Option<Vec<u8>> = if let Some(ver) = version_opt {
                sqlx::query_scalar(
                    "SELECT value_ciphertext FROM team_secret_versions WHERE team_id = $1 AND name = $2 AND version = $3",
                )
                .bind(team_id)
                .bind(name)
                .bind(ver)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?
            } else {
                sqlx::query_scalar(
                    "SELECT value_ciphertext FROM team_secrets WHERE team_id = $1 AND name = $2",
                )
                .bind(team_id)
                .bind(name)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?
            };
            let Some(blob) = blob else {
                if let Some(ver) = version_opt {
                    return Err(format!("missing team secret `{name}@{ver}`"));
                }
                return Err(format!("missing team secret `{name}`"));
            };
            let plain = decrypt_private_key(key, &blob).map_err(|_| "decrypt secret failed")?;
            String::from_utf8(plain).map_err(|_| "secret value is not utf-8")?
        } else {
            v
        };
        out.push((k, resolved, is_secret));
    }
    Ok(out)
}
