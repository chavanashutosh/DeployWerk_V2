//! Team-scoped platform APIs: usage, support, storage, flags, observability, search, edge, sandboxes, agents, RUM, AI gateway, billing.

use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD as B64_STANDARD, Engine as _};
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use axum::body::Body;
use axum::response::Response;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::applications::{
    branch_matches_git_pattern, enqueue_deploy_for_webhook, enqueue_pr_preview_deploy,
    enqueue_pr_preview_destroy, normalize_github_repo_full_name, normalize_git_remote_path,
};
use crate::auth::{hash_api_token_raw, require_principal};
use crate::audit::TeamAuditRow;
use crate::crypto_keys::encrypt_private_key;
use crate::entitlements::require_team_feature;
use crate::error::ApiError;
use crate::rbac::{require_team_member, require_team_mutator};
use crate::webhook_github::verify_github_webhook_hmac_sha256;
use crate::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/teams/{team_id}/usage", get(get_usage))
        .route("/api/v1/teams/{team_id}/support-links", get(get_support).put(put_support))
        .route("/api/v1/teams/{team_id}/storage-backends", get(list_storage).post(create_storage))
        .route(
            "/api/v1/teams/{team_id}/storage-backends/{id}",
            patch(update_storage).delete(delete_storage),
        )
        .route(
            "/api/v1/teams/{team_id}/storage-backends/{id}/test",
            post(test_storage),
        )
        .route("/api/v1/teams/{team_id}/feature-flags", get(list_flags).post(create_flag))
        .route(
            "/api/v1/teams/{team_id}/feature-flags/{id}",
            patch(update_flag).delete(delete_flag),
        )
        .route(
            "/api/v1/teams/{team_id}/observability/summary",
            get(observability_summary),
        )
        .route("/api/v1/teams/{team_id}/health-checks", get(list_checks).post(create_check))
        .route(
            "/api/v1/teams/{team_id}/health-checks/{id}",
            patch(update_check).delete(delete_check),
        )
        .route("/api/v1/teams/{team_id}/search", get(team_search))
        .route(
            "/api/v1/teams/{team_id}/firewall-rules",
            get(list_firewall).post(create_firewall),
        )
        .route(
            "/api/v1/teams/{team_id}/firewall-rules/{id}",
            patch(update_firewall).delete(delete_firewall),
        )
        .route("/api/v1/teams/{team_id}/cdn/purge", post(cdn_purge))
        .route(
            "/api/v1/teams/{team_id}/cdn/purge-requests",
            get(list_cdn_purges),
        )
        .route(
            "/api/v1/teams/{team_id}/edge/traefik-snippet",
            get(traefik_snippet),
        )
        .route(
            "/api/v1/teams/{team_id}/preview-deployments",
            get(list_previews).post(create_preview_manual),
        )
        .route("/api/v1/teams/{team_id}/agents", get(list_agents).post(register_agent))
        .route("/api/v1/teams/{team_id}/agents/{id}", delete(delete_agent))
        .route("/api/v1/teams/{team_id}/rum/config", get(rum_config))
        .route("/api/v1/teams/{team_id}/rum/summary", get(rum_summary))
        .route("/api/v1/teams/{team_id}/ai-gateway/routes", get(list_ai_routes).post(create_ai_route))
        .route(
            "/api/v1/teams/{team_id}/ai-gateway/routes/{id}",
            patch(update_ai_route).delete(delete_ai_route),
        )
        .route(
            "/api/v1/teams/{team_id}/ai-gateway/invoke",
            post(ai_invoke_proxy),
        )
        .route(
            "/api/v1/teams/{team_id}/billing",
            get(get_billing).patch(billing_patch_disabled),
        )
        .route("/api/v1/teams/{team_id}/audit-log", get(list_team_audit_log))
        .route(
            "/api/v1/teams/{team_id}/github-hook-config",
            get(get_github_hook_config).put(put_github_hook_secret),
        )
        .route(
            "/api/v1/teams/{team_id}/gitlab-hook-config",
            get(get_gitlab_hook_config).put(put_gitlab_hook_secret),
        )
        .route(
            "/api/v1/teams/{team_id}/github-app/installations",
            get(list_github_app_installations),
        )
        .route(
            "/api/v1/teams/{team_id}/github-app/install-url",
            get(get_github_app_install_url),
        )
        .route("/api/v1/teams/{team_id}/github-app/installation", post(register_github_app_installation))
        .route(
            "/api/v1/teams/{team_id}/github-app/installation/{installation_id}",
            delete(delete_github_app_installation),
        )
        .route("/api/v1/hooks/github/{team_id}", post(github_hook))
        .route("/api/v1/hooks/gitlab/{team_id}", post(gitlab_hook))
        .route("/api/v1/hooks/github-app", post(github_app_webhook))
        .route("/api/v1/agent/heartbeat", post(agent_heartbeat))
        .route("/api/v1/rum/ingest", post(rum_ingest))
        .route("/api/v1/stripe/webhook", post(stripe_webhook))
        .route("/api/v1/hooks/adyen", post(adyen_webhook))
        .route("/api/v1/mollie/webhook", post(mollie_webhook))
        .route(
            "/api/v1/teams/{team_id}/otlp/v1/traces",
            post(otlp_traces_stub),
        )
        .route(
            "/api/v1/teams/{team_id}/otlp/traces",
            get(list_otlp_traces),
        )
        .route(
            "/api/v1/teams/{team_id}/otlp/traces/{trace_id}",
            get(get_otlp_trace),
        )
        .route(
            "/api/v1/teams/{team_id}/registry/status",
            get(registry_status_stub),
        )
        .route(
            "/api/v1/teams/{team_id}/cost/summary",
            get(cost_summary_stub),
        )
}

#[derive(Deserialize)]
struct AuditQuery {
    #[serde(default = "default_audit_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_audit_limit() -> i64 {
    50
}

async fn list_team_audit_log(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    Query(q): Query<AuditQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamAuditRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    // Conservative: only allow owners/admins (same predicate as “mutate”) to view audit.
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let limit = q.limit.clamp(1, 200);
    let offset = q.offset.max(0);
    let rows: Vec<(
        Uuid,
        Uuid,
        String,
        String,
        Option<Uuid>,
        serde_json::Value,
        Option<String>,
        DateTime<Utc>,
    )> = sqlx::query_as(
        r#"SELECT id, actor_user_id, action, entity_type, entity_id, metadata, source_ip, created_at
           FROM team_audit_log
           WHERE team_id = $1
           ORDER BY created_at DESC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(team_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(
                |(id, actor_user_id, action, entity_type, entity_id, metadata, source_ip, created_at)| TeamAuditRow {
                    id,
                    actor_user_id,
                    action,
                    entity_type,
                    entity_id,
                    metadata,
                    source_ip,
                    created_at,
                },
            )
            .collect(),
    ))
}

/// OTLP trace ingest (Phase 4 MVP): stores raw batch for later processing; returns 202.
async fn otlp_traces_stub(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let max_bytes: usize = std::env::var("DEPLOYWERK_OTLP_MAX_BYTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_000_000);
    if body.len() > max_bytes {
        return Err(ApiError::BadRequest(format!(
            "OTLP payload too large ({} bytes > {})",
            body.len(),
            max_bytes
        )));
    }

    let ct = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .trim()
        .to_string();

    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO otlp_trace_batches (id, team_id, content_type, payload, size_bytes, received_at)
           VALUES ($1,$2,$3,$4,$5,$6)"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(ct)
    .bind(body.as_ref())
    .bind(body.len() as i32)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(StatusCode::ACCEPTED)
}

#[derive(Serialize)]
struct OtlpTraceBatchRow {
    id: Uuid,
    received_at: DateTime<Utc>,
    size_bytes: i32,
    content_type: String,
}

#[derive(Deserialize)]
struct ListOtlpQuery {
    #[serde(default = "default_otlp_limit")]
    limit: i64,
}

fn default_otlp_limit() -> i64 {
    50
}

async fn list_otlp_traces(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    Query(q): Query<ListOtlpQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<OtlpTraceBatchRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let limit = q.limit.clamp(1, 200);
    let rows: Vec<(Uuid, DateTime<Utc>, i32, String)> = sqlx::query_as(
        r#"SELECT id, received_at, size_bytes, content_type
           FROM otlp_trace_batches
           WHERE team_id = $1
           ORDER BY received_at DESC
           LIMIT $2"#,
    )
    .bind(team_id)
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(|(id, received_at, size_bytes, content_type)| OtlpTraceBatchRow {
                id,
                received_at,
                size_bytes,
                content_type,
            })
            .collect(),
    ))
}

async fn get_otlp_trace(
    State(state): State<Arc<AppState>>,
    Path((team_id, trace_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let row: Option<(String, Vec<u8>)> = sqlx::query_as(
        r#"SELECT content_type, payload
           FROM otlp_trace_batches
           WHERE id = $1 AND team_id = $2"#,
    )
    .bind(trace_id)
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((content_type, payload)) = row else {
        return Err(ApiError::NotFound);
    };

    let ct = content_type.trim();
    let ct = if ct.is_empty() {
        "application/octet-stream"
    } else {
        ct
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, ct)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"otlp-trace-{trace_id}.bin\""),
        )
        .body(Body::from(payload))
        .map_err(|_| ApiError::Internal)
}

/// Built-in OCI registry not bundled yet; integrate external registry + CI scan (Phase 5 stub).
async fn registry_status_stub(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;
    Ok(Json(json!({
        "integrated": false,
        "team_id": team_id,
        "hint": "Push images to your registry; optional CVE scan in CI before deploy."
    })))
}

/// Synthetic cost rollup placeholder (Phase 7); wire agent metrics + price table later.
async fn cost_summary_stub(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;
    Ok(Json(json!({
        "team_id": team_id,
        "currency": "USD",
        "synthetic_monthly_estimate": serde_json::Value::Null,
        "note": "Connect usage metering from agents and operator price-per-CPU-hour."
    })))
}

// --- Usage ---

#[derive(Serialize)]
struct UsageResponse {
    period_days: i64,
    deploy_job_count: i64,
    succeeded: i64,
    failed: i64,
}

#[derive(Deserialize)]
struct UsageQuery {
    #[serde(default = "default_usage_days")]
    days: i64,
}

fn default_usage_days() -> i64 {
    30
}

async fn get_usage(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    Query(q): Query<UsageQuery>,
    headers: HeaderMap,
) -> Result<Json<UsageResponse>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let days = q.days.clamp(1, 366);
    let since = Utc::now() - Duration::days(days);

    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::bigint FROM deploy_jobs j
           JOIN applications a ON a.id = j.application_id
           JOIN environments e ON e.id = a.environment_id
           JOIN projects p ON p.id = e.project_id
           WHERE p.team_id = $1 AND j.created_at >= $2"#,
    )
    .bind(team_id)
    .bind(since)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let succeeded: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::bigint FROM deploy_jobs j
           JOIN applications a ON a.id = j.application_id
           JOIN environments e ON e.id = a.environment_id
           JOIN projects p ON p.id = e.project_id
           WHERE p.team_id = $1 AND j.created_at >= $2 AND j.status = 'succeeded'"#,
    )
    .bind(team_id)
    .bind(since)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let failed: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::bigint FROM deploy_jobs j
           JOIN applications a ON a.id = j.application_id
           JOIN environments e ON e.id = a.environment_id
           JOIN projects p ON p.id = e.project_id
           WHERE p.team_id = $1 AND j.created_at >= $2 AND j.status = 'failed'"#,
    )
    .bind(team_id)
    .bind(since)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(UsageResponse {
        period_days: days,
        deploy_job_count: total,
        succeeded,
        failed,
    }))
}

// --- Support links ---

#[derive(Serialize, Deserialize, Default)]
struct SupportLinks {
    docs_url: Option<String>,
    status_url: Option<String>,
    contact_email: Option<String>,
}

async fn get_support(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<SupportLinks>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let row: Option<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT docs_url, status_url, contact_email FROM team_support_links WHERE team_id = $1",
    )
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let (docs_url, status_url, contact_email) = row.unwrap_or((None, None, None));
    Ok(Json(SupportLinks {
        docs_url,
        status_url,
        contact_email,
    }))
}

async fn put_support(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<SupportLinks>,
) -> Result<Json<SupportLinks>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO team_support_links (team_id, docs_url, status_url, contact_email, updated_at)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (team_id) DO UPDATE SET
             docs_url = EXCLUDED.docs_url,
             status_url = EXCLUDED.status_url,
             contact_email = EXCLUDED.contact_email,
             updated_at = EXCLUDED.updated_at"#,
    )
    .bind(team_id)
    .bind(&body.docs_url)
    .bind(&body.status_url)
    .bind(&body.contact_email)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(body))
}

// --- Storage ---

#[derive(Serialize)]
struct StorageRow {
    id: Uuid,
    team_id: Uuid,
    name: String,
    endpoint_url: String,
    bucket: String,
    region: String,
    path_style: bool,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct StorageCreate {
    name: String,
    endpoint_url: String,
    bucket: String,
    #[serde(default)]
    region: String,
    #[serde(default = "default_true")]
    path_style: bool,
    access_key: String,
    secret_key: String,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
struct StoragePatch {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    endpoint_url: Option<String>,
    #[serde(default)]
    bucket: Option<String>,
    #[serde(default)]
    region: Option<String>,
    #[serde(default)]
    path_style: Option<bool>,
    #[serde(default)]
    access_key: Option<String>,
    #[serde(default)]
    secret_key: Option<String>,
}

async fn list_storage(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<StorageRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let rows: Vec<(Uuid, Uuid, String, String, String, String, bool, DateTime<Utc>)> =
        sqlx::query_as(
            r#"SELECT id, team_id, name, endpoint_url, bucket, region, path_style, created_at
               FROM storage_backends WHERE team_id = $1 ORDER BY created_at DESC"#,
        )
        .bind(team_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(
                |(id, tid, name, endpoint_url, bucket, region, path_style, created_at)| StorageRow {
                    id,
                    team_id: tid,
                    name,
                    endpoint_url,
                    bucket,
                    region,
                    path_style,
                    created_at,
                },
            )
            .collect(),
    ))
}

async fn create_storage(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<StorageCreate>,
) -> Result<(StatusCode, Json<StorageRow>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let ak = encrypt_private_key(&state.server_key_encryption_key, body.access_key.as_bytes())
        .map_err(|_| ApiError::Internal)?;
    let sk = encrypt_private_key(&state.server_key_encryption_key, body.secret_key.as_bytes())
        .map_err(|_| ApiError::Internal)?;

    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO storage_backends
           (id, team_id, name, endpoint_url, bucket, region, path_style, access_key_ciphertext, secret_key_ciphertext, created_at)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(body.name.trim())
    .bind(body.endpoint_url.trim())
    .bind(body.bucket.trim())
    .bind(body.region.trim())
    .bind(body.path_style)
    .bind(&ak)
    .bind(&sk)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok((
        StatusCode::CREATED,
        Json(StorageRow {
            id,
            team_id,
            name: body.name,
            endpoint_url: body.endpoint_url,
            bucket: body.bucket,
            region: body.region,
            path_style: body.path_style,
            created_at: now,
        }),
    ))
}

async fn update_storage(
    State(state): State<Arc<AppState>>,
    Path((team_id, sid)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(body): Json<StoragePatch>,
) -> Result<Json<StorageRow>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let row: Option<(
        String,
        String,
        String,
        String,
        bool,
        Vec<u8>,
        Vec<u8>,
        DateTime<Utc>,
    )> = sqlx::query_as(
        r#"SELECT name, endpoint_url, bucket, region, path_style,
                  access_key_ciphertext, secret_key_ciphertext, created_at
           FROM storage_backends WHERE id = $1 AND team_id = $2"#,
    )
    .bind(sid)
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((
        mut name,
        mut endpoint_url,
        mut bucket,
        mut region,
        mut path_style,
        mut ak_ct,
        mut sk_ct,
        created_at,
    )) = row
    else {
        return Err(ApiError::NotFound);
    };

    if let Some(n) = body.name {
        name = n;
    }
    if let Some(u) = body.endpoint_url {
        endpoint_url = u;
    }
    if let Some(b) = body.bucket {
        bucket = b;
    }
    if let Some(r) = body.region {
        region = r;
    }
    if let Some(ps) = body.path_style {
        path_style = ps;
    }
    if let Some(ak) = body.access_key {
        ak_ct = encrypt_private_key(&state.server_key_encryption_key, ak.as_bytes())
            .map_err(|_| ApiError::Internal)?;
    }
    if let Some(sk) = body.secret_key {
        sk_ct = encrypt_private_key(&state.server_key_encryption_key, sk.as_bytes())
            .map_err(|_| ApiError::Internal)?;
    }

    sqlx::query(
        r#"UPDATE storage_backends SET name=$1, endpoint_url=$2, bucket=$3, region=$4, path_style=$5,
           access_key_ciphertext=$6, secret_key_ciphertext=$7 WHERE id=$8 AND team_id=$9"#,
    )
    .bind(&name)
    .bind(&endpoint_url)
    .bind(&bucket)
    .bind(&region)
    .bind(path_style)
    .bind(&ak_ct)
    .bind(&sk_ct)
    .bind(sid)
    .bind(team_id)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(StorageRow {
        id: sid,
        team_id,
        name,
        endpoint_url,
        bucket,
        region,
        path_style,
        created_at,
    }))
}

async fn delete_storage(
    State(state): State<Arc<AppState>>,
    Path((team_id, sid)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let r = sqlx::query("DELETE FROM storage_backends WHERE id = $1 AND team_id = $2")
        .bind(sid)
        .bind(team_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    if r.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn test_storage(
    State(state): State<Arc<AppState>>,
    Path((team_id, sid)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let row: Option<(String,)> =
        sqlx::query_as("SELECT endpoint_url FROM storage_backends WHERE id = $1 AND team_id = $2")
            .bind(sid)
            .bind(team_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;

    let Some((endpoint_url,)) = row else {
        return Err(ApiError::NotFound);
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|_| ApiError::Internal)?;

    let url = endpoint_url.trim_end_matches('/');
    let res = client.get(url).send().await;
    match res {
        Ok(r) => Ok(Json(json!({
            "reachable": true,
            "http_status": r.status().as_u16()
        }))),
        Err(e) => Ok(Json(json!({
            "reachable": false,
            "error": e.to_string()
        }))),
    }
}

// --- Feature flags ---

#[derive(Serialize)]
struct FlagRow {
    id: Uuid,
    team_id: Uuid,
    environment_id: Option<Uuid>,
    flag_key: String,
    value_json: serde_json::Value,
    enabled: bool,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct FlagCreate {
    flag_key: String,
    #[serde(default)]
    environment_id: Option<Uuid>,
    #[serde(default)]
    value_json: serde_json::Value,
    #[serde(default = "default_true")]
    enabled: bool,
}

#[derive(Deserialize)]
struct FlagPatch {
    #[serde(default)]
    value_json: Option<serde_json::Value>,
    #[serde(default)]
    enabled: Option<bool>,
}

async fn ensure_env_in_team(
    pool: &crate::DbPool,
    team_id: Uuid,
    env_id: Uuid,
) -> Result<(), ApiError> {
    let ok: bool = sqlx::query_scalar(
        r#"SELECT EXISTS(
            SELECT 1 FROM environments e JOIN projects p ON p.id = e.project_id
            WHERE e.id = $1 AND p.team_id = $2)"#,
    )
    .bind(env_id)
    .bind(team_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    if ok {
        Ok(())
    } else {
        Err(ApiError::BadRequest("environment not in team".into()))
    }
}

async fn list_flags(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<FlagRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let rows: Vec<(Uuid, Uuid, Option<Uuid>, String, serde_json::Value, bool, DateTime<Utc>)> =
        sqlx::query_as(
            r#"SELECT id, team_id, environment_id, flag_key, value_json, enabled, created_at
               FROM feature_flags WHERE team_id = $1 ORDER BY flag_key"#,
        )
        .bind(team_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(
                |(id, tid, environment_id, flag_key, value_json, enabled, created_at)| FlagRow {
                    id,
                    team_id: tid,
                    environment_id,
                    flag_key,
                    value_json,
                    enabled,
                    created_at,
                },
            )
            .collect(),
    ))
}

async fn create_flag(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<FlagCreate>,
) -> Result<(StatusCode, Json<FlagRow>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    if let Some(eid) = body.environment_id {
        ensure_env_in_team(&state.pool, team_id, eid).await?;
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    let key = body.flag_key.trim().to_string();
    if key.is_empty() {
        return Err(ApiError::BadRequest("empty flag_key".into()));
    }

    sqlx::query(
        r#"INSERT INTO feature_flags (id, team_id, environment_id, flag_key, value_json, enabled, created_at)
           VALUES ($1,$2,$3,$4,$5,$6,$7)"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(body.environment_id)
    .bind(&key)
    .bind(&body.value_json)
    .bind(body.enabled)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        if let Some(db) = e.as_database_error() {
            if db.is_unique_violation() {
                return ApiError::Conflict("flag key already exists for scope".into());
            }
        }
        ApiError::Internal
    })?;

    Ok((
        StatusCode::CREATED,
        Json(FlagRow {
            id,
            team_id,
            environment_id: body.environment_id,
            flag_key: key,
            value_json: body.value_json,
            enabled: body.enabled,
            created_at: now,
        }),
    ))
}

async fn update_flag(
    State(state): State<Arc<AppState>>,
    Path((team_id, fid)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(body): Json<FlagPatch>,
) -> Result<Json<FlagRow>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let row: Option<(String, serde_json::Value, bool, Option<Uuid>, DateTime<Utc>)> = sqlx::query_as(
        "SELECT flag_key, value_json, enabled, environment_id, created_at FROM feature_flags WHERE id = $1 AND team_id = $2",
    )
    .bind(fid)
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((flag_key, mut value_json, mut enabled, environment_id, created_at)) = row else {
        return Err(ApiError::NotFound);
    };

    if let Some(v) = body.value_json {
        value_json = v;
    }
    if let Some(e) = body.enabled {
        enabled = e;
    }

    sqlx::query(
        "UPDATE feature_flags SET value_json = $1, enabled = $2 WHERE id = $3 AND team_id = $4",
    )
    .bind(&value_json)
    .bind(enabled)
    .bind(fid)
    .bind(team_id)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(FlagRow {
        id: fid,
        team_id,
        environment_id,
        flag_key,
        value_json,
        enabled,
        created_at,
    }))
}

async fn delete_flag(
    State(state): State<Arc<AppState>>,
    Path((team_id, fid)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let r = sqlx::query("DELETE FROM feature_flags WHERE id = $1 AND team_id = $2")
        .bind(fid)
        .bind(team_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    if r.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// --- Observability ---

#[derive(Serialize)]
struct ObsSummary {
    checks_total: i64,
    checks_ok_recent: i64,
    last_failures: Vec<serde_json::Value>,
}

async fn observability_summary(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<ObsSummary>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM health_checks WHERE team_id = $1")
            .bind(team_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;

    let since = Utc::now() - Duration::hours(1);
    let ok_recent: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(DISTINCT r.check_id)::bigint FROM health_check_results r
           JOIN health_checks c ON c.id = r.check_id
           WHERE c.team_id = $1 AND r.checked_at >= $2 AND r.ok = TRUE"#,
    )
    .bind(team_id)
    .bind(since)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let fails: Vec<(Uuid, bool, Option<i32>, Option<String>, DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT r.check_id, r.ok, r.latency_ms, r.error_message, r.checked_at
           FROM health_check_results r
           JOIN health_checks c ON c.id = r.check_id
           WHERE c.team_id = $1 AND r.ok = FALSE
           ORDER BY r.checked_at DESC LIMIT 20"#,
    )
    .bind(team_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let last_failures = fails
        .into_iter()
        .map(|(check_id, ok, latency_ms, error_message, checked_at)| {
            json!({
                "check_id": check_id,
                "ok": ok,
                "latency_ms": latency_ms,
                "error_message": error_message,
                "checked_at": checked_at
            })
        })
        .collect();

    Ok(Json(ObsSummary {
        checks_total: total,
        checks_ok_recent: ok_recent,
        last_failures,
    }))
}

#[derive(Serialize)]
struct HealthCheckRow {
    id: Uuid,
    team_id: Uuid,
    name: String,
    target_url: String,
    interval_seconds: i32,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct CheckCreate {
    name: String,
    target_url: String,
    #[serde(default = "default_interval")]
    interval_seconds: i32,
}

fn default_interval() -> i32 {
    60
}

#[derive(Deserialize)]
struct CheckPatch {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    target_url: Option<String>,
    #[serde(default)]
    interval_seconds: Option<i32>,
}

async fn list_checks(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<HealthCheckRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let rows: Vec<(Uuid, Uuid, String, String, i32, DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT id, team_id, name, target_url, interval_seconds, created_at
           FROM health_checks WHERE team_id = $1 ORDER BY name"#,
    )
    .bind(team_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(
                |(id, tid, name, target_url, interval_seconds, created_at)| HealthCheckRow {
                    id,
                    team_id: tid,
                    name,
                    target_url,
                    interval_seconds,
                    created_at,
                },
            )
            .collect(),
    ))
}

async fn create_check(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<CheckCreate>,
) -> Result<(StatusCode, Json<HealthCheckRow>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let iv = body.interval_seconds.clamp(15, 86_400);
    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO health_checks (id, team_id, name, target_url, interval_seconds, created_at)
           VALUES ($1,$2,$3,$4,$5,$6)"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(body.name.trim())
    .bind(body.target_url.trim())
    .bind(iv)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok((
        StatusCode::CREATED,
        Json(HealthCheckRow {
            id,
            team_id,
            name: body.name,
            target_url: body.target_url,
            interval_seconds: iv,
            created_at: now,
        }),
    ))
}

async fn update_check(
    State(state): State<Arc<AppState>>,
    Path((team_id, cid)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(body): Json<CheckPatch>,
) -> Result<Json<HealthCheckRow>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let row: Option<(String, String, i32, DateTime<Utc>)> = sqlx::query_as(
        "SELECT name, target_url, interval_seconds, created_at FROM health_checks WHERE id = $1 AND team_id = $2",
    )
    .bind(cid)
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((mut name, mut target_url, mut interval_seconds, created_at)) = row else {
        return Err(ApiError::NotFound);
    };

    if let Some(n) = body.name {
        name = n;
    }
    if let Some(u) = body.target_url {
        target_url = u;
    }
    if let Some(i) = body.interval_seconds {
        interval_seconds = i.clamp(15, 86_400);
    }

    sqlx::query(
        "UPDATE health_checks SET name=$1, target_url=$2, interval_seconds=$3 WHERE id=$4 AND team_id=$5",
    )
    .bind(&name)
    .bind(&target_url)
    .bind(interval_seconds)
    .bind(cid)
    .bind(team_id)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(HealthCheckRow {
        id: cid,
        team_id,
        name,
        target_url,
        interval_seconds,
        created_at,
    }))
}

async fn delete_check(
    State(state): State<Arc<AppState>>,
    Path((team_id, cid)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let r = sqlx::query("DELETE FROM health_checks WHERE id = $1 AND team_id = $2")
        .bind(cid)
        .bind(team_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    if r.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// --- Search ---

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

#[derive(Serialize)]
struct SearchHit {
    kind: &'static str,
    id: Uuid,
    title: String,
    subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    environment_id: Option<Uuid>,
}

async fn team_search(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    Query(q): Query<SearchQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<SearchHit>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let needle = format!("%{}%", q.q.trim().replace('%', "\\%").replace('_', "\\_"));
    if needle == "%%" {
        return Ok(Json(vec![]));
    }

    let mut hits = Vec::new();

    let projects: Vec<(Uuid, String, String)> = sqlx::query_as(
        r#"SELECT id, name, slug FROM projects
           WHERE team_id = $1 AND (LOWER(name) LIKE LOWER($2) ESCAPE '\' OR LOWER(slug) LIKE LOWER($2) ESCAPE '\') LIMIT 20"#,
    )
    .bind(team_id)
    .bind(&needle)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    for (id, name, slug) in projects {
        hits.push(SearchHit {
            kind: "project",
            id,
            title: name,
            subtitle: Some(slug),
            project_id: Some(id),
            environment_id: None,
        });
    }

    let envs: Vec<(Uuid, String, String, String, Uuid)> = sqlx::query_as(
        r#"SELECT e.id, e.name, e.slug, p.name, e.project_id FROM environments e
           JOIN projects p ON p.id = e.project_id
           WHERE p.team_id = $1 AND (LOWER(e.name) LIKE LOWER($2) ESCAPE '\' OR LOWER(e.slug) LIKE LOWER($2) ESCAPE '\') LIMIT 20"#,
    )
    .bind(team_id)
    .bind(&needle)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    for (id, name, slug, pn, project_id) in envs {
        hits.push(SearchHit {
            kind: "environment",
            id,
            title: name,
            subtitle: Some(format!("{pn} / {slug}")),
            project_id: Some(project_id),
            environment_id: Some(id),
        });
    }

    let apps: Vec<(Uuid, String, String, String, String, Uuid, Uuid)> = sqlx::query_as(
        r#"SELECT a.id, a.name, a.slug, e.name, p.name, e.project_id, e.id FROM applications a
           JOIN environments e ON e.id = a.environment_id
           JOIN projects p ON p.id = e.project_id
           WHERE p.team_id = $1 AND (LOWER(a.name) LIKE LOWER($2) ESCAPE '\' OR LOWER(a.slug) LIKE LOWER($2) ESCAPE '\') LIMIT 20"#,
    )
    .bind(team_id)
    .bind(&needle)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    for (id, name, slug, en, pn, project_id, environment_id) in apps {
        hits.push(SearchHit {
            kind: "application",
            id,
            title: name,
            subtitle: Some(format!("{pn} / {en} / {slug}")),
            project_id: Some(project_id),
            environment_id: Some(environment_id),
        });
    }

    let servers: Vec<(Uuid, String, String)> = sqlx::query_as(
        r#"SELECT id, name, host FROM servers
           WHERE team_id = $1 AND (LOWER(name) LIKE LOWER($2) ESCAPE '\' OR LOWER(host) LIKE LOWER($2) ESCAPE '\') LIMIT 20"#,
    )
    .bind(team_id)
    .bind(&needle)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    for (id, name, host) in servers {
        hits.push(SearchHit {
            kind: "server",
            id,
            title: name,
            subtitle: Some(host),
            project_id: None,
            environment_id: None,
        });
    }

    Ok(Json(hits))
}

// --- Edge: firewall + CDN ---

#[derive(Serialize)]
struct FirewallRow {
    id: Uuid,
    team_id: Uuid,
    label: String,
    cidr: String,
    enabled: bool,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct FirewallCreate {
    #[serde(default)]
    label: String,
    cidr: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

#[derive(Deserialize)]
struct FirewallPatch {
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    cidr: Option<String>,
    #[serde(default)]
    enabled: Option<bool>,
}

async fn list_firewall(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<FirewallRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let rows: Vec<(Uuid, Uuid, String, String, bool, DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT id, team_id, label, cidr, enabled, created_at
           FROM team_firewall_rules WHERE team_id = $1 ORDER BY created_at"#,
    )
    .bind(team_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(
                |(id, tid, label, cidr, enabled, created_at)| FirewallRow {
                    id,
                    team_id: tid,
                    label,
                    cidr,
                    enabled,
                    created_at,
                },
            )
            .collect(),
    ))
}

fn validate_cidr_notation(cidr: &str) -> Result<(), &'static str> {
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        return Err("cidr must look like address/prefix (e.g. 203.0.113.0/24)");
    }
    let prefix: u32 = parts[1].parse().map_err(|_| "invalid prefix length")?;
    if prefix > 128 {
        return Err("prefix length out of range");
    }
    let addr = parts[0].trim();
    if addr.is_empty() {
        return Err("address part empty");
    }
    Ok(())
}

async fn create_firewall(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<FirewallCreate>,
) -> Result<(StatusCode, Json<FirewallRow>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let cidr = body.cidr.trim();
    if cidr.is_empty() {
        return Err(ApiError::BadRequest("cidr required".into()));
    }
    if let Err(e) = validate_cidr_notation(cidr) {
        return Err(ApiError::BadRequest(e.into()));
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO team_firewall_rules (id, team_id, label, cidr, enabled, created_at)
           VALUES ($1,$2,$3,$4,$5,$6)"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(body.label.trim())
    .bind(cidr)
    .bind(body.enabled)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok((
        StatusCode::CREATED,
        Json(FirewallRow {
            id,
            team_id,
            label: body.label,
            cidr: cidr.to_string(),
            enabled: body.enabled,
            created_at: now,
        }),
    ))
}

async fn update_firewall(
    State(state): State<Arc<AppState>>,
    Path((team_id, rid)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(body): Json<FirewallPatch>,
) -> Result<Json<FirewallRow>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let row: Option<(String, String, bool, DateTime<Utc>)> = sqlx::query_as(
        "SELECT label, cidr, enabled, created_at FROM team_firewall_rules WHERE id = $1 AND team_id = $2",
    )
    .bind(rid)
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((mut label, mut cidr, mut enabled, created_at)) = row else {
        return Err(ApiError::NotFound);
    };

    if let Some(l) = body.label {
        label = l;
    }
    if let Some(c) = body.cidr {
        let t = c.trim();
        if let Err(e) = validate_cidr_notation(t) {
            return Err(ApiError::BadRequest(e.into()));
        }
        cidr = t.to_string();
    }
    if let Some(e) = body.enabled {
        enabled = e;
    }

    sqlx::query(
        "UPDATE team_firewall_rules SET label=$1, cidr=$2, enabled=$3 WHERE id=$4 AND team_id=$5",
    )
    .bind(&label)
    .bind(&cidr)
    .bind(enabled)
    .bind(rid)
    .bind(team_id)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(FirewallRow {
        id: rid,
        team_id,
        label,
        cidr,
        enabled,
        created_at,
    }))
}

async fn delete_firewall(
    State(state): State<Arc<AppState>>,
    Path((team_id, rid)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let r = sqlx::query("DELETE FROM team_firewall_rules WHERE id = $1 AND team_id = $2")
        .bind(rid)
        .bind(team_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    if r.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct PurgeBody {
    #[serde(default)]
    paths: String,
}

async fn cdn_purge(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<PurgeBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let id = Uuid::new_v4();
    let now = Utc::now();
    let initial_detail = if state.cdn_purge_webhook_url.is_some() {
        "Recorded; notifying edge webhook…"
    } else {
        "Recorded locally (no DEPLOYWERK_CDN_PURGE_WEBHOOK_URL configured)"
    };
    sqlx::query(
        r#"INSERT INTO cdn_purge_requests (id, team_id, paths, status, detail, created_at)
           VALUES ($1,$2,$3,'done',$4,$5)"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(&body.paths)
    .bind(initial_detail)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    if let Some(ref hook_url) = state.cdn_purge_webhook_url {
        let pool = state.pool.clone();
        let url = hook_url.clone();
        let paths = body.paths.clone();
        let tid = team_id;
        let rid = id;
        tokio::spawn(async move {
            let client = match reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
            {
                Ok(c) => c,
                Err(_) => {
                    let _ = sqlx::query(
                        "UPDATE cdn_purge_requests SET detail = $1 WHERE id = $2",
                    )
                    .bind("Recorded; edge webhook client init failed")
                    .bind(rid)
                    .execute(&pool)
                    .await;
                    return;
                }
            };
            let payload = json!({
                "team_id": tid,
                "purge_id": rid,
                "paths": paths,
            });
            let detail = match client.post(&url).json(&payload).send().await {
                Ok(resp) if resp.status().is_success() => {
                    format!("Recorded; edge webhook OK ({})", resp.status().as_u16())
                }
                Ok(resp) => format!(
                    "Recorded; edge webhook returned HTTP {}",
                    resp.status().as_u16()
                ),
                Err(e) => format!("Recorded; edge webhook error: {e}"),
            };
            let _ = sqlx::query("UPDATE cdn_purge_requests SET detail = $1 WHERE id = $2")
                .bind(&detail)
                .bind(rid)
                .execute(&pool)
                .await;
        });
    }

    Ok(Json(json!({ "id": id, "status": "done" })))
}

async fn list_cdn_purges(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let rows: Vec<(Uuid, String, String, Option<String>, DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT id, paths, status, detail, created_at FROM cdn_purge_requests
           WHERE team_id = $1 ORDER BY created_at DESC LIMIT 50"#,
    )
    .bind(team_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(|(id, paths, status, detail, created_at)| {
                json!({
                    "id": id,
                    "paths": paths,
                    "status": status,
                    "detail": detail,
                    "created_at": created_at
                })
            })
            .collect(),
    ))
}

async fn traefik_snippet(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT cidr FROM team_firewall_rules WHERE team_id = $1 AND enabled = TRUE ORDER BY created_at",
    )
    .bind(team_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let mut yaml = String::from(
        "# DeployWerk — IP allowlist snippet for Traefik (ipWhiteList middleware)\n# Apply via dynamic provider or labels; verify CIDRs before production.\nhttp:\n  middlewares:\n    deploywerk-allowlist:\n      ipWhiteList:\n        sourceRange:\n",
    );
    yaml.push_str("          - \"127.0.0.1/32\"\n");
    for (cidr,) in rows {
        yaml.push_str(&format!("          - \"{cidr}\"\n"));
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/yaml; charset=utf-8")
        .body(Body::from(yaml))
        .map_err(|_| ApiError::Internal)
}

// --- Sandboxes / previews ---

#[derive(Serialize)]
struct PreviewRow {
    id: Uuid,
    team_id: Uuid,
    branch: String,
    commit_sha: String,
    status: String,
    meta: serde_json::Value,
    created_at: DateTime<Utc>,
}

async fn list_previews(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<PreviewRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let rows: Vec<(Uuid, Uuid, String, String, String, serde_json::Value, DateTime<Utc>)> =
        sqlx::query_as(
            r#"SELECT id, team_id, branch, commit_sha, status, meta, created_at
               FROM preview_deployments WHERE team_id = $1 ORDER BY created_at DESC LIMIT 100"#,
        )
        .bind(team_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(
                |(id, tid, branch, commit_sha, status, meta, created_at)| PreviewRow {
                    id,
                    team_id: tid,
                    branch,
                    commit_sha,
                    status,
                    meta,
                    created_at,
                },
            )
            .collect(),
    ))
}

#[derive(Deserialize)]
struct PreviewStub {
    #[serde(default)]
    branch: String,
    #[serde(default)]
    commit_sha: String,
}

async fn create_preview_manual(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<PreviewStub>,
) -> Result<(StatusCode, Json<PreviewRow>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let id = Uuid::new_v4();
    let now = Utc::now();
    let meta = json!({ "source": "manual" });
    sqlx::query(
        r#"INSERT INTO preview_deployments (id, team_id, branch, commit_sha, status, meta, created_at)
           VALUES ($1,$2,$3,$4,'active',$5,$6)"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(body.branch.trim())
    .bind(body.commit_sha.trim())
    .bind(&meta)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok((
        StatusCode::CREATED,
        Json(PreviewRow {
            id,
            team_id,
            branch: body.branch,
            commit_sha: body.commit_sha,
            status: "active".into(),
            meta,
            created_at: now,
        }),
    ))
}

#[derive(Serialize)]
struct GithubHookConfigResponse {
    /// Path to configure in GitHub (prepend your API public URL).
    pub hook_path: String,
    pub secret_configured: bool,
}

async fn get_github_hook_config(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<GithubHookConfigResponse>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let secret_row: Option<Option<String>> = sqlx::query_scalar(
        "SELECT github_webhook_secret FROM teams WHERE id = $1",
    )
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some(secret_opt) = secret_row else {
        return Err(ApiError::NotFound);
    };

    let configured = secret_opt
        .as_ref()
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    Ok(Json(GithubHookConfigResponse {
        hook_path: format!("/api/v1/hooks/github/{team_id}"),
        secret_configured: configured,
    }))
}

#[derive(Deserialize)]
struct GithubHookSecretBody {
    #[serde(default)]
    secret: Option<String>,
}

async fn put_github_hook_secret(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<GithubHookSecretBody>,
) -> Result<Json<GithubHookConfigResponse>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let sec = body.secret.map(|s| s.trim().to_string());
    let sec = sec.filter(|s| !s.is_empty());

    sqlx::query("UPDATE teams SET github_webhook_secret = $1 WHERE id = $2")
        .bind(&sec)
        .bind(team_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    Ok(Json(GithubHookConfigResponse {
        hook_path: format!("/api/v1/hooks/github/{team_id}"),
        secret_configured: sec.is_some(),
    }))
}

async fn github_hook(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    let secret_row: Option<Option<String>> = sqlx::query_scalar(
        "SELECT github_webhook_secret FROM teams WHERE id = $1",
    )
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some(secret_opt) = secret_row else {
        return Err(ApiError::NotFound);
    };

    if let Some(sec) = secret_opt.as_ref().filter(|s| !s.is_empty()) {
        let sig = headers
            .get("x-hub-signature-256")
            .or_else(|| headers.get("X-Hub-Signature-256"))
            .and_then(|v| v.to_str().ok());
        verify_github_webhook_hmac_sha256(sec.as_str(), &body, sig)?;
    }

    let event = headers
        .get("x-github-event")
        .or_else(|| headers.get("X-GitHub-Event"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if event == "ping" {
        return Ok(Json(json!({ "ok": true, "event": "ping" })));
    }

    if event != "push" {
        return Ok(Json(json!({ "ok": true, "skipped": event })));
    }

    let v: serde_json::Value =
        serde_json::from_slice(&body).map_err(|_| ApiError::BadRequest("invalid json".into()))?;

    if v.get("head_commit").is_none() {
        return Ok(Json(json!({ "ok": true, "skipped": "no head_commit" })));
    }

    let repo = v
        .get("repository")
        .and_then(|r| r.get("full_name"))
        .and_then(|x| x.as_str())
        .ok_or(ApiError::BadRequest("missing repository.full_name".into()))?;

    let repo_norm = normalize_github_repo_full_name(repo).ok_or(ApiError::BadRequest("could not normalize repository name".into()))?;

    let sha = v
        .get("head_commit")
        .and_then(|h| h.get("id"))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    let ref_s = v.get("ref").and_then(|x| x.as_str()).unwrap_or("");
    let branch = ref_s.strip_prefix("refs/heads/").unwrap_or(ref_s);

    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        r#"SELECT a.id, a.git_branch_pattern FROM applications a
           JOIN environments e ON e.id = a.environment_id
           JOIN projects p ON p.id = e.project_id
           WHERE p.team_id = $1
             AND a.auto_deploy_on_push = TRUE
             AND a.git_repo_full_name IS NOT NULL
             AND LOWER(TRIM(a.git_repo_full_name)) = LOWER(TRIM($2))"#,
    )
    .bind(team_id)
    .bind(&repo_norm)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let mut job_ids: Vec<Uuid> = Vec::new();
    for (app_id, pattern) in rows {
        if !branch_matches_git_pattern(&pattern, branch) {
            continue;
        }
        match enqueue_deploy_for_webhook(
            state.clone(),
            app_id,
            team_id,
            ref_s.to_string(),
            sha.to_string(),
        )
        .await
        {
            Ok(r) => job_ids.push(r.job_id),
            Err(e) => tracing::warn!(?e, %app_id, "webhook deploy enqueue failed"),
        }
    }

    let preview_id = Uuid::new_v4();
    let now = Utc::now();
    let meta = json!({
        "repository": repo,
        "ref": ref_s,
        "branch": branch,
        "source": "github_webhook",
        "deploy_job_ids": job_ids,
    });
    sqlx::query(
        r#"INSERT INTO preview_deployments (id, team_id, branch, commit_sha, status, meta, created_at)
           VALUES ($1,$2,$3,$4,'active',$5,$6)"#,
    )
    .bind(preview_id)
    .bind(team_id)
    .bind(branch)
    .bind(sha)
    .bind(&meta)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        json!({ "ok": true, "preview_id": preview_id, "deploy_jobs": job_ids.len(), "job_ids": job_ids }),
    ))
}

// --- GitLab webhook ---

#[derive(Serialize)]
struct GitlabHookConfigResponse {
    pub hook_path: String,
    pub secret_configured: bool,
}

async fn get_gitlab_hook_config(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<GitlabHookConfigResponse>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let secret_row: Option<Option<String>> = sqlx::query_scalar(
        "SELECT gitlab_webhook_secret FROM teams WHERE id = $1",
    )
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some(secret_opt) = secret_row else {
        return Err(ApiError::NotFound);
    };

    let configured = secret_opt
        .as_ref()
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    Ok(Json(GitlabHookConfigResponse {
        hook_path: format!("/api/v1/hooks/gitlab/{team_id}"),
        secret_configured: configured,
    }))
}

#[derive(Deserialize)]
struct GitlabHookSecretBody {
    #[serde(default)]
    secret: Option<String>,
}

async fn put_gitlab_hook_secret(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<GitlabHookSecretBody>,
) -> Result<Json<GitlabHookConfigResponse>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let sec = body.secret.map(|s| s.trim().to_string());
    let sec = sec.filter(|s| !s.is_empty());

    sqlx::query("UPDATE teams SET gitlab_webhook_secret = $1 WHERE id = $2")
        .bind(&sec)
        .bind(team_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    Ok(Json(GitlabHookConfigResponse {
        hook_path: format!("/api/v1/hooks/gitlab/{team_id}"),
        secret_configured: sec.is_some(),
    }))
}

async fn gitlab_hook(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    let secret_row: Option<Option<String>> = sqlx::query_scalar(
        "SELECT gitlab_webhook_secret FROM teams WHERE id = $1",
    )
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some(secret_opt) = secret_row else {
        return Err(ApiError::NotFound);
    };

    if let Some(sec) = secret_opt.as_ref().filter(|s| !s.is_empty()) {
        let token = headers
            .get("x-gitlab-token")
            .or_else(|| headers.get("X-Gitlab-Token"))
            .and_then(|v| v.to_str().ok());
        let got = token.unwrap_or("");
        if got.len() != sec.len() || got.as_bytes().ct_eq(sec.as_bytes()).unwrap_u8() != 1 {
            return Err(ApiError::Unauthorized);
        }
    }

    let v: serde_json::Value =
        serde_json::from_slice(&body).map_err(|_| ApiError::BadRequest("invalid json".into()))?;

    if v.get("object_kind").and_then(|x| x.as_str()) != Some("push") {
        let sk = v
            .get("object_kind")
            .and_then(|x| x.as_str())
            .unwrap_or("");
        return Ok(Json(json!({ "ok": true, "skipped": sk })));
    }

    let ref_s = v.get("ref").and_then(|x| x.as_str()).unwrap_or("");
    let branch = ref_s.strip_prefix("refs/heads/").unwrap_or(ref_s);

    let sha = v
        .get("checkout_sha")
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty() && !s.chars().all(|c| c == '0'))
        .or_else(|| v.get("after").and_then(|x| x.as_str()))
        .unwrap_or("");
    if sha.is_empty() || sha.chars().all(|c| c == '0') {
        return Ok(Json(json!({ "ok": true, "skipped": "no commit sha" })));
    }

    let path = v
        .get("project")
        .and_then(|p| p.get("path_with_namespace"))
        .and_then(|x| x.as_str())
        .ok_or(ApiError::BadRequest("missing project.path_with_namespace".into()))?;
    let repo_norm = normalize_git_remote_path(path);

    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        r#"SELECT a.id, a.git_branch_pattern FROM applications a
           JOIN environments e ON e.id = a.environment_id
           JOIN projects p ON p.id = e.project_id
           WHERE p.team_id = $1
             AND a.auto_deploy_on_push = TRUE
             AND a.git_repo_full_name IS NOT NULL
             AND LOWER(TRIM(a.git_repo_full_name)) = LOWER(TRIM($2))"#,
    )
    .bind(team_id)
    .bind(&repo_norm)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let mut job_ids: Vec<Uuid> = Vec::new();
    for (app_id, pattern) in rows {
        if !branch_matches_git_pattern(&pattern, branch) {
            continue;
        }
        match enqueue_deploy_for_webhook(
            state.clone(),
            app_id,
            team_id,
            ref_s.to_string(),
            sha.to_string(),
        )
        .await
        {
            Ok(r) => job_ids.push(r.job_id),
            Err(e) => tracing::warn!(?e, %app_id, "gitlab webhook deploy enqueue failed"),
        }
    }

    let preview_id = Uuid::new_v4();
    let now = Utc::now();
    let meta = json!({
        "repository": path,
        "ref": ref_s,
        "branch": branch,
        "source": "gitlab_webhook",
        "deploy_job_ids": job_ids,
    });
    sqlx::query(
        r#"INSERT INTO preview_deployments (id, team_id, branch, commit_sha, status, meta, created_at)
           VALUES ($1,$2,$3,$4,'active',$5,$6)"#,
    )
    .bind(preview_id)
    .bind(team_id)
    .bind(branch)
    .bind(sha)
    .bind(&meta)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        json!({ "ok": true, "preview_id": preview_id, "deploy_jobs": job_ids.len(), "job_ids": job_ids }),
    ))
}

// --- GitHub App (PR previews) ---

#[derive(Deserialize)]
struct RegisterGithubAppInstallationBody {
    installation_id: i64,
    #[serde(default)]
    account_login: Option<String>,
}

async fn register_github_app_installation(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<RegisterGithubAppInstallationBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    if body.installation_id <= 0 {
        return Err(ApiError::BadRequest("installation_id required".into()));
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    let login = body
        .account_login
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    sqlx::query(
        r#"INSERT INTO github_app_installations (id, team_id, installation_id, account_login, created_at)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (installation_id) DO UPDATE SET
             team_id = EXCLUDED.team_id,
             account_login = EXCLUDED.account_login"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(body.installation_id)
    .bind(&login)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok((
        StatusCode::CREATED,
        Json(json!({ "ok": true, "team_id": team_id, "installation_id": body.installation_id })),
    ))
}

async fn delete_github_app_installation(
    State(state): State<Arc<AppState>>,
    Path((team_id, installation_id)): Path<(Uuid, i64)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let n = sqlx::query(
        "DELETE FROM github_app_installations WHERE team_id = $1 AND installation_id = $2",
    )
    .bind(team_id)
    .bind(installation_id)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?
    .rows_affected();

    if n == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
struct GithubAppInstallationListRow {
    id: Uuid,
    installation_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    account_login: Option<String>,
    created_at: DateTime<Utc>,
}

async fn list_github_app_installations(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<GithubAppInstallationListRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let rows: Vec<(Uuid, i64, Option<String>, DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT id, installation_id, account_login, created_at
           FROM github_app_installations
           WHERE team_id = $1
           ORDER BY created_at DESC"#,
    )
    .bind(team_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let out = rows
        .into_iter()
        .map(|(id, installation_id, account_login, created_at)| GithubAppInstallationListRow {
            id,
            installation_id,
            account_login,
            created_at,
        })
        .collect();

    Ok(Json(out))
}

async fn get_github_app_install_url(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let slug = state
        .github_app_slug
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ApiError::BadRequest(
                "GITHUB_APP_SLUG is not set on the API; add it to generate an install link.".into(),
            )
        })?;

    let url = format!("https://github.com/apps/{slug}/installations/new?state={team_id}");
    Ok(Json(json!({ "url": url })))
}

async fn github_app_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Some(sec) = state
        .github_app_webhook_secret
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    else {
        return Err(ApiError::NotFound);
    };

    let sig = headers
        .get("x-hub-signature-256")
        .or_else(|| headers.get("X-Hub-Signature-256"))
        .and_then(|v| v.to_str().ok());
    verify_github_webhook_hmac_sha256(sec, &body, sig)?;

    let event = headers
        .get("x-github-event")
        .or_else(|| headers.get("X-GitHub-Event"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if event == "ping" {
        return Ok(Json(json!({ "ok": true, "event": "ping" })));
    }

    let v: serde_json::Value =
        serde_json::from_slice(&body).map_err(|_| ApiError::BadRequest("invalid json".into()))?;

    match event {
        "pull_request" => handle_github_app_pull_request(state, v).await,
        "installation" => handle_github_app_installation_event(state, v).await,
        _ => Ok(Json(json!({ "ok": true, "skipped": event }))),
    }
}

async fn handle_github_app_installation_event(
    state: Arc<AppState>,
    v: serde_json::Value,
) -> Result<Json<serde_json::Value>, ApiError> {
    let action = v.get("action").and_then(|x| x.as_str()).unwrap_or("");
    if action != "deleted" {
        return Ok(Json(json!({ "ok": true, "skipped": action })));
    }
    let inst_id = v
        .get("installation")
        .and_then(|i| i.get("id"))
        .and_then(|x| x.as_i64())
        .ok_or_else(|| ApiError::BadRequest("missing installation id".into()))?;

    sqlx::query("DELETE FROM github_app_installations WHERE installation_id = $1")
        .bind(inst_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    Ok(Json(json!({ "ok": true, "deleted_installation_id": inst_id })))
}

async fn handle_github_app_pull_request(
    state: Arc<AppState>,
    v: serde_json::Value,
) -> Result<Json<serde_json::Value>, ApiError> {
    let action = v.get("action").and_then(|x| x.as_str()).unwrap_or("");
    let pr = v
        .get("pull_request")
        .ok_or_else(|| ApiError::BadRequest("missing pull_request".into()))?;
    let pr_number = pr
        .get("number")
        .and_then(|x| x.as_i64())
        .ok_or_else(|| ApiError::BadRequest("missing pull_request.number".into()))? as i32;
    let head_sha = pr
        .get("head")
        .and_then(|h| h.get("sha"))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    let head_ref = pr
        .get("head")
        .and_then(|h| h.get("ref"))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    let base_sha = pr
        .get("base")
        .and_then(|b| b.get("sha"))
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let repo_full = pr
        .get("base")
        .and_then(|b| b.get("repo"))
        .and_then(|r| r.get("full_name"))
        .and_then(|x| x.as_str())
        .or_else(|| {
            v.get("repository")
                .and_then(|r| r.get("full_name"))
                .and_then(|x| x.as_str())
        })
        .ok_or_else(|| ApiError::BadRequest("missing repository full_name".into()))?;
    let repo_norm = normalize_github_repo_full_name(repo_full)
        .ok_or_else(|| ApiError::BadRequest("could not normalize repository name".into()))?;
    let inst_id = v
        .get("installation")
        .and_then(|i| i.get("id"))
        .and_then(|x| x.as_i64())
        .ok_or_else(|| ApiError::BadRequest("missing installation".into()))?;

    let team_row: Option<Uuid> = sqlx::query_scalar(
        "SELECT team_id FROM github_app_installations WHERE installation_id = $1",
    )
    .bind(inst_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some(team_id) = team_row else {
        return Ok(Json(json!({ "ok": true, "skipped": "unknown_installation" })));
    };

    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        r#"SELECT a.id, a.git_branch_pattern FROM applications a
           JOIN environments e ON e.id = a.environment_id
           JOIN projects p ON p.id = e.project_id
           WHERE p.team_id = $1
             AND a.pr_preview_enabled = TRUE
             AND a.git_repo_full_name IS NOT NULL
             AND LOWER(TRIM(a.git_repo_full_name)) = LOWER(TRIM($2))"#,
    )
    .bind(team_id)
    .bind(&repo_norm)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    if action == "closed" {
        let mut job_ids: Vec<Uuid> = Vec::new();
        for (app_id, pattern) in rows {
            if !branch_matches_git_pattern(&pattern, head_ref) {
                continue;
            }
            match enqueue_pr_preview_destroy(state.clone(), app_id, team_id, pr_number).await {
                Ok(r) => job_ids.push(r.job_id),
                Err(e) => tracing::warn!(?e, %app_id, "pr preview destroy enqueue failed"),
            }
        }
        return Ok(Json(json!({
            "ok": true,
            "action": "closed",
            "destroy_jobs": job_ids.len(),
            "job_ids": job_ids
        })));
    }

    if !matches!(action, "opened" | "synchronize" | "reopened") {
        return Ok(Json(json!({ "ok": true, "skipped": action })));
    }

    if head_sha.is_empty() {
        return Ok(Json(json!({ "ok": true, "skipped": "no head sha" })));
    }

    let mut job_ids: Vec<Uuid> = Vec::new();
    for (app_id, pattern) in rows {
        if !branch_matches_git_pattern(&pattern, head_ref) {
            continue;
        }
        match enqueue_pr_preview_deploy(
            state.clone(),
            app_id,
            team_id,
            format!("refs/heads/{head_ref}"),
            head_sha.to_string(),
            base_sha.clone(),
            pr_number,
        )
        .await
        {
            Ok(r) => job_ids.push(r.job_id),
            Err(e) => tracing::warn!(?e, %app_id, "pr preview deploy enqueue failed"),
        }
    }

    Ok(Json(json!({
        "ok": true,
        "action": action,
        "deploy_jobs": job_ids.len(),
        "job_ids": job_ids
    })))
}

// --- Agents ---

#[derive(Serialize)]
struct AgentRow {
    id: Uuid,
    team_id: Uuid,
    name: String,
    version: Option<String>,
    meta: serde_json::Value,
    last_seen_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct AgentRegister {
    name: String,
}

#[derive(Serialize)]
struct AgentRegisterResponse {
    id: Uuid,
    token: String,
}

async fn list_agents(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<AgentRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let rows: Vec<(Uuid, Uuid, String, Option<String>, serde_json::Value, Option<DateTime<Utc>>, DateTime<Utc>)> =
        sqlx::query_as(
            r#"SELECT id, team_id, name, version, meta, last_seen_at, created_at
               FROM team_agents WHERE team_id = $1 ORDER BY created_at DESC"#,
        )
        .bind(team_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(
                |(id, tid, name, version, meta, last_seen_at, created_at)| AgentRow {
                    id,
                    team_id: tid,
                    name,
                    version,
                    meta,
                    last_seen_at,
                    created_at,
                },
            )
            .collect(),
    ))
}

async fn register_agent(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<AgentRegister>,
) -> Result<(StatusCode, Json<AgentRegisterResponse>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let raw = format!("dw_agent_{}", Uuid::new_v4().simple());
    let token_hash = hash_api_token_raw(&raw);
    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO team_agents (id, team_id, name, token_hash, version, meta, last_seen_at, created_at)
           VALUES ($1,$2,$3,$4,NULL,'{}',NULL,$5)"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(body.name.trim())
    .bind(&token_hash)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok((
        StatusCode::CREATED,
        Json(AgentRegisterResponse { id, token: raw }),
    ))
}

async fn delete_agent(
    State(state): State<Arc<AppState>>,
    Path((team_id, aid)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let r = sqlx::query("DELETE FROM team_agents WHERE id = $1 AND team_id = $2")
        .bind(aid)
        .bind(team_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    if r.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct HeartbeatBody {
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    meta: Option<serde_json::Value>,
}

async fn agent_heartbeat(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<HeartbeatBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(ApiError::Unauthorized)?;

    let h = hash_api_token_raw(auth);
    let row: Option<Uuid> = sqlx::query_scalar("SELECT id FROM team_agents WHERE token_hash = $1")
        .bind(&h)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    let Some(aid) = row else {
        return Err(ApiError::Unauthorized);
    };

    let now = Utc::now();
    let meta = body.meta.unwrap_or_else(|| json!({}));
    sqlx::query(
        r#"UPDATE team_agents SET last_seen_at = $1, version = COALESCE($2, version), meta = $3 WHERE id = $4"#,
    )
    .bind(now)
    .bind(&body.version)
    .bind(sqlx::types::Json(meta))
    .bind(aid)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(json!({ "ok": true })))
}

// --- RUM ---

async fn ensure_rum_secret(pool: &crate::DbPool, team_id: Uuid) -> Result<String, ApiError> {
    let cur: Option<String> = sqlx::query_scalar("SELECT rum_ingest_secret FROM teams WHERE id = $1")
        .bind(team_id)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    if let Some(s) = cur.filter(|x| !x.is_empty()) {
        return Ok(s);
    }

    let s = format!("rum_{}", Uuid::new_v4().simple());
    sqlx::query("UPDATE teams SET rum_ingest_secret = $1 WHERE id = $2")
        .bind(&s)
        .bind(team_id)
        .execute(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    Ok(s)
}

async fn rum_config(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;
    require_team_feature(&state.pool, team_id, "rum").await?;

    let secret = ensure_rum_secret(&state.pool, team_id).await?;
    Ok(Json(json!({ "ingest_secret": secret })))
}

#[derive(Serialize)]
struct RumSummary {
    period_days: i64,
    by_metric: Vec<(String, f64)>,
    sample_count: i64,
}

async fn rum_summary(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<RumSummary>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;
    require_team_feature(&state.pool, team_id, "rum").await?;

    let days = 7i64;
    let since = Utc::now() - Duration::days(days);

    let rows: Vec<(String, f64)> = sqlx::query_as(
        r#"SELECT metric_name, AVG(metric_value)::float8 FROM rum_events
           WHERE team_id = $1 AND recorded_at >= $2
           GROUP BY metric_name ORDER BY metric_name"#,
    )
    .bind(team_id)
    .bind(since)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM rum_events WHERE team_id = $1 AND recorded_at >= $2",
    )
    .bind(team_id)
    .bind(since)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(RumSummary {
        period_days: days,
        by_metric: rows,
        sample_count: count,
    }))
}

#[derive(Deserialize)]
struct RumIngestRow {
    page_path: String,
    metric_name: String,
    metric_value: f64,
}

async fn rum_ingest(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<RumIngestRow>,
) -> Result<StatusCode, ApiError> {
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(ApiError::Unauthorized)?;

    let team_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM teams WHERE rum_ingest_secret = $1",
    )
    .bind(auth)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?
    .flatten();

    let Some(team_id) = team_id else {
        return Err(ApiError::Unauthorized);
    };

    if !crate::entitlements::team_has_feature(&state.pool, team_id, "rum").await? {
        return Err(ApiError::Forbidden);
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO rum_events (id, team_id, page_path, metric_name, metric_value, recorded_at)
           VALUES ($1,$2,$3,$4,$5,$6)"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(body.page_path.trim())
    .bind(body.metric_name.trim())
    .bind(body.metric_value)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(StatusCode::NO_CONTENT)
}

// --- AI Gateway ---

#[derive(Serialize)]
struct AiRouteRow {
    id: Uuid,
    team_id: Uuid,
    name: String,
    path_prefix: String,
    upstream_url: String,
    enabled: bool,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct AiRouteCreate {
    name: String,
    path_prefix: String,
    upstream_url: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

#[derive(Deserialize)]
struct AiRoutePatch {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    path_prefix: Option<String>,
    #[serde(default)]
    upstream_url: Option<String>,
    #[serde(default)]
    enabled: Option<bool>,
}

#[derive(Deserialize)]
struct AiInvokeBody {
    route_id: Uuid,
    #[serde(default)]
    path_suffix: String,
    #[serde(default)]
    body: serde_json::Value,
}

async fn list_ai_routes(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<AiRouteRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;
    require_team_feature(&state.pool, team_id, "ai_gateway").await?;

    let rows: Vec<(Uuid, Uuid, String, String, String, bool, DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT id, team_id, name, path_prefix, upstream_url, enabled, created_at
           FROM ai_gateway_routes WHERE team_id = $1 ORDER BY name"#,
    )
    .bind(team_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(
                |(id, tid, name, path_prefix, upstream_url, enabled, created_at)| AiRouteRow {
                    id,
                    team_id: tid,
                    name,
                    path_prefix,
                    upstream_url,
                    enabled,
                    created_at,
                },
            )
            .collect(),
    ))
}

async fn create_ai_route(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<AiRouteCreate>,
) -> Result<(StatusCode, Json<AiRouteRow>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;
    require_team_feature(&state.pool, team_id, "ai_gateway").await?;

    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO ai_gateway_routes (id, team_id, name, path_prefix, upstream_url, enabled, created_at)
           VALUES ($1,$2,$3,$4,$5,$6,$7)"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(body.name.trim())
    .bind(body.path_prefix.trim())
    .bind(body.upstream_url.trim())
    .bind(body.enabled)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok((
        StatusCode::CREATED,
        Json(AiRouteRow {
            id,
            team_id,
            name: body.name,
            path_prefix: body.path_prefix,
            upstream_url: body.upstream_url,
            enabled: body.enabled,
            created_at: now,
        }),
    ))
}

async fn update_ai_route(
    State(state): State<Arc<AppState>>,
    Path((team_id, rid)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(body): Json<AiRoutePatch>,
) -> Result<Json<AiRouteRow>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;
    require_team_feature(&state.pool, team_id, "ai_gateway").await?;

    let row: Option<(String, String, String, bool, DateTime<Utc>)> = sqlx::query_as(
        "SELECT name, path_prefix, upstream_url, enabled, created_at FROM ai_gateway_routes WHERE id = $1 AND team_id = $2",
    )
    .bind(rid)
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((mut name, mut path_prefix, mut upstream_url, mut enabled, created_at)) = row else {
        return Err(ApiError::NotFound);
    };

    if let Some(n) = body.name {
        name = n;
    }
    if let Some(p) = body.path_prefix {
        path_prefix = p;
    }
    if let Some(u) = body.upstream_url {
        upstream_url = u;
    }
    if let Some(e) = body.enabled {
        enabled = e;
    }

    sqlx::query(
        "UPDATE ai_gateway_routes SET name=$1, path_prefix=$2, upstream_url=$3, enabled=$4 WHERE id=$5 AND team_id=$6",
    )
    .bind(&name)
    .bind(&path_prefix)
    .bind(&upstream_url)
    .bind(enabled)
    .bind(rid)
    .bind(team_id)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(AiRouteRow {
        id: rid,
        team_id,
        name,
        path_prefix,
        upstream_url,
        enabled,
        created_at,
    }))
}

async fn delete_ai_route(
    State(state): State<Arc<AppState>>,
    Path((team_id, rid)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;
    require_team_feature(&state.pool, team_id, "ai_gateway").await?;

    let r = sqlx::query("DELETE FROM ai_gateway_routes WHERE id = $1 AND team_id = $2")
        .bind(rid)
        .bind(team_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    if r.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn ai_invoke_proxy(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<AiInvokeBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;
    require_team_feature(&state.pool, team_id, "ai_gateway").await?;

    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT upstream_url, path_prefix FROM ai_gateway_routes WHERE id = $1 AND team_id = $2 AND enabled = TRUE",
    )
    .bind(body.route_id)
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((upstream, _prefix)) = row else {
        return Err(ApiError::NotFound);
    };

    let base = upstream.trim_end_matches('/');
    let suf = body.path_suffix.trim_start_matches('/');
    let url = if suf.is_empty() {
        base.to_string()
    } else {
        format!("{base}/{suf}")
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|_| ApiError::Internal)?;

    let res = client
        .post(&url)
        .json(&body.body)
        .header("X-DeployWerk-Proxy", "ai-gateway")
        .send()
        .await
        .map_err(|_| ApiError::Internal)?;

    let status = res.status().as_u16();
    let text = res.text().await.unwrap_or_default();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap_or(json!({ "raw": text }));

    Ok(Json(json!({
        "upstream_status": status,
        "upstream_response": parsed
    })))
}

// --- Billing ---

#[derive(Serialize, Deserialize)]
struct BillingRow {
    plan_name: String,
    status: String,
    #[serde(default = "default_payment_none")]
    payment_provider: String,
    #[serde(default)]
    provider_customer_id: Option<String>,
    /// Legacy field; mirrored with `provider_customer_id` for Stripe-era clients.
    #[serde(default)]
    stripe_customer_id: Option<String>,
}

fn default_payment_none() -> String {
    "none".into()
}

async fn billing_patch_disabled() -> Result<(), ApiError> {
    Err(ApiError::ForbiddenReason(
        "billing_operator_managed: use the super admin API /api/v1/admin/billing/{team_id}".into(),
    ))
}

async fn get_billing(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<BillingRow>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let row: Option<(Option<String>, String, String, String, Option<String>)> = sqlx::query_as(
        r#"SELECT stripe_customer_id, plan_name, status, payment_provider, provider_customer_id
           FROM team_billing WHERE team_id = $1"#,
    )
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let (stripe_customer_id, plan_name, status, payment_provider, provider_customer_id) = row
        .map(|(s, p, st, pp, pc)| (s, p, st, pp, pc))
        .unwrap_or_else(|| {
            (
                None,
                "free".into(),
                "inactive".into(),
                "none".into(),
                None,
            )
        });

    Ok(Json(BillingRow {
        plan_name,
        status,
        payment_provider,
        provider_customer_id: provider_customer_id.clone().or_else(|| stripe_customer_id.clone()),
        stripe_customer_id,
    }))
}

fn stripe_webhook_signing_key(secret: &str) -> Result<Vec<u8>, ApiError> {
    let s = secret.trim();
    let raw = s
        .strip_prefix("whsec_")
        .ok_or_else(|| ApiError::BadRequest("STRIPE_WEBHOOK_SECRET must start with whsec_".into()))?;
    B64_STANDARD
        .decode(raw.trim())
        .map_err(|_| ApiError::Unauthorized)
}

fn verify_stripe_signature(secret: &str, body: &[u8], sig_header: &str) -> Result<(), ApiError> {
    let key = stripe_webhook_signing_key(secret)?;
    let mut ts: Option<&str> = None;
    let mut v1_sigs: Vec<&str> = Vec::new();
    for part in sig_header.split(',') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("t=") {
            ts = Some(v);
        } else if let Some(v) = part.strip_prefix("v1=") {
            v1_sigs.push(v);
        }
    }
    let t = ts.ok_or(ApiError::Unauthorized)?;
    let mut signed = Vec::with_capacity(t.len() + 1 + body.len());
    signed.extend_from_slice(t.as_bytes());
    signed.push(b'.');
    signed.extend_from_slice(body);

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(&key).map_err(|_| ApiError::Internal)?;
    mac.update(&signed);
    let expected = mac.finalize().into_bytes();

    for sig_hex in v1_sigs {
        let Ok(got) = hex::decode(sig_hex) else {
            continue;
        };
        if got.len() == expected.len()
            && expected.as_slice().ct_eq(got.as_slice()).unwrap_u8() == 1
        {
            return Ok(());
        }
    }
    Err(ApiError::Unauthorized)
}

async fn stripe_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    if let Some(ref secret) = state.stripe_webhook_secret {
        let sig = headers
            .get("Stripe-Signature")
            .and_then(|v| v.to_str().ok())
            .ok_or(ApiError::Unauthorized)?;
        verify_stripe_signature(secret, body.as_ref(), sig)?;
    } else {
        tracing::info!(
            stripe_webhook_bytes = body.len(),
            "stripe webhook received (STRIPE_WEBHOOK_SECRET unset; signature not verified)"
        );
        return Ok(Json(json!({ "received": true })));
    }

    let v: serde_json::Value =
        serde_json::from_slice(body.as_ref()).unwrap_or_else(|_| json!({}));
    let typ = v
        .get("type")
        .and_then(|x| x.as_str())
        .unwrap_or("unknown");
    tracing::info!(event_type = typ, "stripe webhook event verified");

    let obj = v.pointer("/data/object").cloned().unwrap_or_else(|| json!({}));
    let team_id = stripe_resolve_team_id(&obj);

    let ev_id = Uuid::new_v4();
    let now = Utc::now();
    let ref_id = obj
        .get("id")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    sqlx::query(
        r#"INSERT INTO billing_events (id, team_id, provider, event_code, psp_reference, merchant_reference, payload, created_at)
           VALUES ($1,$2,'stripe',$3,$4,$5,$6,$7)"#,
    )
    .bind(ev_id)
    .bind(team_id)
    .bind(typ)
    .bind(&ref_id)
    .bind(&ref_id)
    .bind(v.clone())
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    if let Some(tid) = team_id {
        let mut sync = false;
        let mut billing_status = "active";
        match typ {
            "checkout.session.completed" | "invoice.paid" => {
                sync = true;
                billing_status = "active";
            }
            "customer.subscription.deleted" => {
                sync = true;
                billing_status = "cancelled";
            }
            "customer.subscription.updated" => {
                sync = true;
                let sub_st = obj.get("status").and_then(|s| s.as_str()).unwrap_or("");
                billing_status = if sub_st == "active" || sub_st == "trialing" {
                    "active"
                } else {
                    "inactive"
                };
            }
            _ => {}
        }
        if sync {
            let plan_name = obj
                .pointer("/metadata/plan")
                .and_then(|x| x.as_str())
                .unwrap_or("stripe")
                .to_string();
            let cust = obj
                .get("customer")
                .and_then(|c| c.as_str().map(|s| s.to_string()));
            sqlx::query(
                r#"INSERT INTO team_billing (team_id, stripe_customer_id, plan_name, status, updated_at, payment_provider, provider_customer_id, billing_sync_json)
                   VALUES ($1, $2, $3, $4, $5, 'stripe', $6, $7)
                   ON CONFLICT (team_id) DO UPDATE SET
                     stripe_customer_id = COALESCE(EXCLUDED.stripe_customer_id, team_billing.stripe_customer_id),
                     plan_name = EXCLUDED.plan_name,
                     status = EXCLUDED.status,
                     updated_at = EXCLUDED.updated_at,
                     payment_provider = 'stripe',
                     provider_customer_id = COALESCE(team_billing.provider_customer_id, EXCLUDED.provider_customer_id),
                     billing_sync_json = EXCLUDED.billing_sync_json"#,
            )
            .bind(tid)
            .bind(&cust)
            .bind(&plan_name)
            .bind(billing_status)
            .bind(now)
            .bind(cust.clone())
            .bind(obj.clone())
            .execute(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
        }
    }

    Ok(Json(json!({ "received": true, "type": typ, "team_id": team_id })))
}

fn parse_team_id_from_merchant_reference(mref: &str) -> Option<Uuid> {
    let s = mref.trim();
    if let Ok(u) = Uuid::parse_str(s) {
        return Some(u);
    }
    s.strip_prefix("deploywerk_team_")
        .and_then(|rest| Uuid::parse_str(rest).ok())
}

fn stripe_resolve_team_id(obj: &serde_json::Value) -> Option<Uuid> {
    if let Some(s) = obj.pointer("/metadata/team_id").and_then(|v| v.as_str()) {
        let t = s.trim();
        if let Ok(u) = Uuid::parse_str(t) {
            return Some(u);
        }
        if let Some(u) = parse_team_id_from_merchant_reference(t) {
            return Some(u);
        }
    }
    if let Some(s) = obj.get("client_reference_id").and_then(|v| v.as_str()) {
        let t = s.trim();
        if let Ok(u) = Uuid::parse_str(t) {
            return Some(u);
        }
        if let Some(u) = parse_team_id_from_merchant_reference(t) {
            return Some(u);
        }
    }
    None
}

fn adyen_hmac_payload(item: &serde_json::Value) -> String {
    let psp = item.get("pspReference").and_then(|v| v.as_str()).unwrap_or("");
    let orig = item
        .get("originalReference")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let macct = item
        .get("merchantAccountCode")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let mref = item
        .get("merchantReference")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let value = item
        .get("amount")
        .and_then(|a| a.get("value"))
        .and_then(|v| {
            v.as_i64()
                .map(|n| n.to_string())
                .or_else(|| v.as_str().map(|s| s.to_string()))
        })
        .unwrap_or_default();
    let currency = item
        .get("amount")
        .and_then(|a| a.get("currency"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let event_code = item.get("eventCode").and_then(|v| v.as_str()).unwrap_or("");
    let success = item
        .get("success")
        .map(|v| {
            if let Some(b) = v.as_str() {
                b.to_lowercase()
            } else if let Some(b) = v.as_bool() {
                if b {
                    "true".into()
                } else {
                    "false".into()
                }
            } else {
                "false".into()
            }
        })
        .unwrap_or_else(|| "false".into());
    format!("{psp}:{orig}:{macct}:{mref}:{value}:{currency}:{event_code}:{success}")
}

fn verify_adyen_hmac(key_hex: &str, item: &serde_json::Value) -> Result<(), ApiError> {
    let expected = item
        .pointer("/additionalData/hmacSignature")
        .and_then(|v| v.as_str())
        .ok_or(ApiError::Unauthorized)?;
    let payload = adyen_hmac_payload(item);
    let key_bytes = hex::decode(key_hex.trim()).map_err(|_| ApiError::Unauthorized)?;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(&key_bytes).map_err(|_| ApiError::Internal)?;
    mac.update(payload.as_bytes());
    let computed = B64_STANDARD.encode(mac.finalize().into_bytes());
    if computed == expected {
        Ok(())
    } else {
        Err(ApiError::Unauthorized)
    }
}

async fn adyen_webhook(
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    let v: serde_json::Value =
        serde_json::from_slice(body.as_ref()).unwrap_or_else(|_| json!({}));

    let items = v
        .get("notificationItems")
        .and_then(|n| n.as_array())
        .cloned()
        .unwrap_or_default();

    if items.is_empty() {
        tracing::warn!("adyen webhook: no notificationItems");
        return Ok(Json(json!({ "received": true })));
    }

    for wrapped in &items {
        let item = wrapped
            .get("NotificationRequestItem")
            .cloned()
            .unwrap_or(json!({}));

        if let Some(ref key) = state.adyen_hmac_key_hex {
            if item.pointer("/additionalData/hmacSignature").is_some() {
                verify_adyen_hmac(key, &item)?;
            } else {
                tracing::warn!("adyen webhook: missing hmacSignature while ADYEN_HMAC_KEY_HEX is set");
                return Err(ApiError::Unauthorized);
            }
        } else {
            tracing::info!("adyen webhook (ADYEN_HMAC_KEY_HEX unset; HMAC not verified)");
        }

        let event_code = item
            .get("eventCode")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let mref = item
            .get("merchantReference")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let psp = item
            .get("pspReference")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string());

        let team_id = parse_team_id_from_merchant_reference(&mref);

        let ev_id = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            r#"INSERT INTO billing_events (id, team_id, provider, event_code, psp_reference, merchant_reference, payload, created_at)
               VALUES ($1,$2,'adyen',$3,$4,$5,$6,$7)"#,
        )
        .bind(ev_id)
        .bind(team_id)
        .bind(&event_code)
        .bind(&psp)
        .bind(&mref)
        .bind(item.clone())
        .bind(now)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

        if let Some(tid) = team_id {
            let success = item
                .get("success")
                .map(|x| {
                    x.as_str()
                        .map(|s| s.eq_ignore_ascii_case("true"))
                        .unwrap_or_else(|| x.as_bool().unwrap_or(false))
                })
                .unwrap_or(false);

            let mut status = if success { "active" } else { "failed" }.to_string();
            if event_code == "CANCELLATION" || event_code == "CANCEL_OR_REFUND" {
                status = "cancelled".into();
            }

            let plan_name = item
                .pointer("/additionalData/metadata.plan")
                .and_then(|x| x.as_str())
                .unwrap_or("adyen")
                .to_string();

            sqlx::query(
                r#"INSERT INTO team_billing (team_id, stripe_customer_id, plan_name, status, updated_at, payment_provider, provider_customer_id, billing_sync_json)
                   VALUES ($1, NULL, $2, $3, $4, 'adyen', $5, $6)
                   ON CONFLICT (team_id) DO UPDATE SET
                     plan_name = EXCLUDED.plan_name,
                     status = EXCLUDED.status,
                     updated_at = EXCLUDED.updated_at,
                     payment_provider = 'adyen',
                     provider_customer_id = COALESCE(team_billing.provider_customer_id, EXCLUDED.provider_customer_id),
                     billing_sync_json = EXCLUDED.billing_sync_json"#,
            )
            .bind(tid)
            .bind(&plan_name)
            .bind(&status)
            .bind(now)
            .bind(&mref)
            .bind(item.clone())
            .execute(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
        }
    }

    Ok(Json(json!({ "received": true, "count": items.len() })))
}

// --- Mollie ---

async fn mollie_webhook(
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Some(ref api_key) = state.mollie_api_key else {
        tracing::info!(bytes = body.len(), "mollie webhook received (MOLLIE_API_KEY unset)");
        return Ok(Json(json!({ "received": true, "verified": false })));
    };

    let v: serde_json::Value =
        serde_json::from_slice(body.as_ref()).map_err(|_| ApiError::BadRequest("invalid json".into()))?;
    let payment_id = v
        .get("id")
        .and_then(|x| x.as_str())
        .ok_or_else(|| ApiError::BadRequest("missing payment id".into()))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|_| ApiError::Internal)?;

    let url = format!("https://api.mollie.com/v2/payments/{payment_id}");
    let res = client
        .get(&url)
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", api_key.trim()),
        )
        .send()
        .await
        .map_err(|_| ApiError::Internal)?;

    if !res.status().is_success() {
        tracing::warn!(status = %res.status(), "mollie payment fetch failed");
        return Err(ApiError::Unauthorized);
    }

    let pay: serde_json::Value = res.json().await.map_err(|_| ApiError::Internal)?;
    let status_str = pay
        .get("status")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();

    let mut team_id = pay
        .get("metadata")
        .and_then(|m| m.get("team_id"))
        .and_then(|x| x.as_str())
        .and_then(|s| Uuid::parse_str(s.trim()).ok());
    if team_id.is_none() {
        if let Some(desc) = pay.get("description").and_then(|x| x.as_str()) {
            team_id = parse_team_id_from_merchant_reference(desc);
        }
    }
    if team_id.is_none() {
        if let Some(r) = pay.get("reference").and_then(|x| x.as_str()) {
            team_id = parse_team_id_from_merchant_reference(r);
        }
    }

    let customer_id = pay
        .get("customerId")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());

    let plan_name = pay
        .pointer("/metadata/plan")
        .and_then(|x| x.as_str())
        .unwrap_or("mollie")
        .to_string();

    let ev_id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO billing_events (id, team_id, provider, event_code, psp_reference, merchant_reference, payload, created_at)
           VALUES ($1,$2,'mollie',$3,$4,$5,$6,$7)"#,
    )
    .bind(ev_id)
    .bind(team_id)
    .bind(&status_str)
    .bind(payment_id)
    .bind(payment_id)
    .bind(pay.clone())
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    if let Some(tid) = team_id {
        let billing_status = match status_str.as_str() {
            "paid" | "authorized" => "active",
            "failed" | "expired" | "canceled" => "inactive",
            _ => "pending",
        };
        let prov_id = customer_id.as_deref().unwrap_or(payment_id);
        sqlx::query(
            r#"INSERT INTO team_billing (team_id, stripe_customer_id, plan_name, status, updated_at, payment_provider, provider_customer_id, billing_sync_json)
               VALUES ($1, NULL, $2, $3, $4, 'mollie', $5, $6)
               ON CONFLICT (team_id) DO UPDATE SET
                 plan_name = EXCLUDED.plan_name,
                 status = EXCLUDED.status,
                 updated_at = EXCLUDED.updated_at,
                 payment_provider = 'mollie',
                 provider_customer_id = COALESCE(team_billing.provider_customer_id, EXCLUDED.provider_customer_id),
                 billing_sync_json = EXCLUDED.billing_sync_json"#,
        )
        .bind(tid)
        .bind(&plan_name)
        .bind(billing_status)
        .bind(now)
        .bind(prov_id)
        .bind(pay.clone())
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    }

    Ok(Json(json!({
        "received": true,
        "verified": true,
        "payment_id": payment_id,
        "status": status_str
    })))
}

// --- Health check background worker ---

pub async fn run_health_check_loop(pool: crate::DbPool) {
    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(30));
    loop {
        ticker.tick().await;
        if let Err(e) = run_health_checks_once(&pool).await {
            tracing::warn!(?e, "health check sweep failed");
        }
    }
}

// --- Housekeeping background worker (previews, low-risk cleanup) ---

/// Best-effort cleanup to keep placeholder tables bounded.
///
/// This does not attempt to infer whether a preview is still “live” at the edge; it only applies
/// conservative retention rules to rows in `preview_deployments`.
pub async fn run_housekeeping_loop(pool: crate::DbPool) {
    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(60 * 30));
    loop {
        ticker.tick().await;
        if let Err(e) = preview_retention_sweep(&pool).await {
            tracing::warn!(?e, "housekeeping sweep failed");
        }
        if let Err(e) = otlp_retention_sweep(&pool).await {
            tracing::warn!(?e, "OTLP retention sweep failed");
        }
    }
}

async fn preview_retention_sweep(pool: &crate::DbPool) -> Result<(), ApiError> {
    // Keep “active” previews longer; tear-down/error rows can be dropped earlier.
    let active_days: i64 = std::env::var("DEPLOYWERK_PREVIEW_RETENTION_DAYS_ACTIVE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let inactive_days: i64 = std::env::var("DEPLOYWERK_PREVIEW_RETENTION_DAYS_INACTIVE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7);

    let active_days = active_days.clamp(1, 365);
    let inactive_days = inactive_days.clamp(1, 365);

    let inactive_cutoff = Utc::now() - Duration::days(inactive_days);
    let active_cutoff = Utc::now() - Duration::days(active_days);

    // Inactive rows: safe to delete quickly.
    sqlx::query(
        r#"DELETE FROM preview_deployments
           WHERE status <> 'active' AND created_at < $1"#,
    )
    .bind(inactive_cutoff)
    .execute(pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    // Active rows older than cutoff: mark as torn_down to signal UI they are stale, then delete
    // via the inactive rule on a later sweep.
    sqlx::query(
        r#"UPDATE preview_deployments
           SET status = 'torn_down'
           WHERE status = 'active' AND created_at < $1"#,
    )
    .bind(active_cutoff)
    .execute(pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(())
}

async fn otlp_retention_sweep(pool: &crate::DbPool) -> Result<(), ApiError> {
    let days: i64 = std::env::var("DEPLOYWERK_OTLP_RETENTION_DAYS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7);
    let days = days.clamp(1, 90);
    let cutoff = Utc::now() - Duration::days(days);
    sqlx::query("DELETE FROM otlp_trace_batches WHERE received_at < $1")
        .bind(cutoff)
        .execute(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    Ok(())
}

async fn run_health_checks_once(pool: &crate::DbPool) -> Result<(), ApiError> {
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, target_url FROM health_checks",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|_| ApiError::Internal)?;

    for (check_id, url) in rows {
        let start = std::time::Instant::now();
        let res = client.get(&url).send().await;
        let elapsed = start.elapsed().as_millis() as i32;
        let now = Utc::now();
        let id = Uuid::new_v4();

        match res {
            Ok(r) if r.status().is_success() => {
                sqlx::query(
                    r#"INSERT INTO health_check_results (id, check_id, ok, latency_ms, error_message, checked_at)
                       VALUES ($1,$2,TRUE,$3,NULL,$4)"#,
                )
                .bind(id)
                .bind(check_id)
                .bind(elapsed)
                .bind(now)
                .execute(pool)
                .await
                .ok();
            }
            Ok(r) => {
                let msg = format!("HTTP {}", r.status());
                sqlx::query(
                    r#"INSERT INTO health_check_results (id, check_id, ok, latency_ms, error_message, checked_at)
                       VALUES ($1,$2,FALSE,$3,$4,$5)"#,
                )
                .bind(id)
                .bind(check_id)
                .bind(elapsed)
                .bind(&msg)
                .bind(now)
                .execute(pool)
                .await
                .ok();
            }
            Err(e) => {
                sqlx::query(
                    r#"INSERT INTO health_check_results (id, check_id, ok, latency_ms, error_message, checked_at)
                       VALUES ($1,$2,FALSE,NULL,$3,$4)"#,
                )
                .bind(id)
                .bind(check_id)
                .bind(e.to_string())
                .bind(now)
                .execute(pool)
                .await
                .ok();
            }
        }
    }

    Ok(())
}
