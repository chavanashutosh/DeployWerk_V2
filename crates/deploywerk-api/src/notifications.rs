//! Outbound webhooks / notification endpoints (deploy events).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::DbPool;
use uuid::Uuid;

use crate::auth::require_principal;
use crate::error::ApiError;
use crate::mail::{self, SmtpSettings};
use crate::rbac::{require_team_member, require_team_mutator};
use crate::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/v1/teams/{team_id}/notification-endpoints",
            get(list_endpoints).post(create_endpoint),
        )
        .route(
            "/api/v1/teams/{team_id}/notification-endpoints/{endpoint_id}",
            patch(update_endpoint).delete(delete_endpoint),
        )
        .route(
            "/api/v1/teams/{team_id}/notification-endpoints/{endpoint_id}/test",
            post(test_endpoint),
        )
}

#[derive(Serialize)]
pub struct NotificationEndpointRow {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_url: Option<String>,
    pub events: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct CreateEndpointBody {
    name: String,
    kind: String,
    target_url: String,
    #[serde(default)]
    events: Option<String>,
}

#[derive(Deserialize)]
struct UpdateEndpointBody {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    target_url: Option<String>,
    #[serde(default)]
    events: Option<String>,
    #[serde(default)]
    enabled: Option<bool>,
}

fn parse_events_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

fn events_contains(events_csv: &str, event: &str) -> bool {
    parse_events_csv(events_csv)
        .iter()
        .any(|e| e == event)
}

/// Telegram: `target_url` stores `CHAT_ID|https://api.telegram.org/bot<token>/sendMessage` (pipe-separated).
fn is_email_recipient(raw: &str) -> bool {
    let s = raw.trim();
    !s.is_empty()
        && s.contains('@')
        && !s.contains(' ')
        && !s.starts_with("http")
        && s.len() <= 320
}

fn parse_telegram_target(raw: &str) -> Option<(String, String)> {
    let (a, b) = raw.split_once('|')?;
    let chat_id = a.trim();
    let api_url = b.trim();
    if chat_id.is_empty() || !api_url.starts_with("http") {
        return None;
    }
    Some((chat_id.to_string(), api_url.to_string()))
}

fn notification_post_body(
    kind: &str,
    target_url: &str,
    payload: &serde_json::Value,
    _name: &str,
) -> Result<(String, serde_json::Value), ApiError> {
    match kind {
        "discord_webhook" => {
            let text = format!(
                "**DeployWerk** `{}` — {}\njob `{}` app **{}** (`{}`)",
                payload.get("event").and_then(|v| v.as_str()).unwrap_or("event"),
                payload
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("—"),
                payload.get("job_id").and_then(|v| v.as_str()).unwrap_or(""),
                payload
                    .get("application_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
                payload
                    .get("application_slug")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
            );
            let body = json!({
                "username": "DeployWerk",
                "embeds": [{
                    "title": "DeployWerk",
                    "description": text,
                    "color": 3447003_i32
                }]
            });
            Ok((target_url.to_string(), body))
        }
        "telegram" => {
            let (chat_id, api_url) =
                parse_telegram_target(target_url).ok_or_else(|| {
                    ApiError::BadRequest(
                        "telegram target_url must be CHAT_ID|https://api.telegram.org/bot…/sendMessage"
                            .into(),
                    )
                })?;
            let text = format!(
                "<b>DeployWerk</b> {}\nstatus: {}\njob: {}\napp: {} ({})",
                payload.get("event").and_then(|v| v.as_str()).unwrap_or(""),
                payload
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("—"),
                payload.get("job_id").and_then(|v| v.as_str()).unwrap_or(""),
                payload
                    .get("application_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
                payload
                    .get("application_slug")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
            );
            let body = json!({
                "chat_id": chat_id,
                "text": text,
                "parse_mode": "HTML",
                "disable_web_page_preview": true
            });
            Ok((api_url, body))
        }
        _ => Ok((target_url.to_string(), payload.clone())),
    }
}

pub fn spawn_deploy_notifications(
    pool: DbPool,
    team_id: Uuid,
    job_id: Uuid,
    application_id: Uuid,
    application_name: &str,
    application_slug: &str,
    event: &'static str,
    status: Option<String>,
    smtp: Option<SmtpSettings>,
) {
    let app_name = application_name.to_string();
    let slug = application_slug.to_string();
    tokio::spawn(async move {
        let _ = fan_out_webhooks(
            &pool,
            team_id,
            event,
            json!({
                "event": event,
                "job_id": job_id,
                "application_id": application_id,
                "application_name": app_name,
                "application_slug": slug,
                "status": status,
                "ts": Utc::now().to_rfc3339(),
            }),
            smtp,
        )
        .await;
    });
}

async fn fan_out_webhooks(
    pool: &DbPool,
    team_id: Uuid,
    event: &str,
    payload: serde_json::Value,
    smtp: Option<SmtpSettings>,
) -> Result<(), ApiError> {
    let rows: Vec<(Uuid, String, String, String, String)> = sqlx::query_as(
        r#"SELECT id, kind, target_url, events, name FROM notification_endpoints
           WHERE team_id = $1 AND enabled = TRUE"#,
    )
    .bind(team_id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|_| ApiError::Internal)?;

    for (_id, kind, target_url, events_csv, name) in rows {
        if !events_contains(&events_csv, event) {
            continue;
        }
        if kind == "email" {
            let Some(ref smtp_cfg) = smtp else {
                tracing::warn!("email notification skipped: SMTP not configured on instance");
                continue;
            };
            let to = target_url.trim();
            if !is_email_recipient(to) {
                tracing::warn!(%to, "email notification skipped: invalid target_url");
                continue;
            }
            let (subject, body) = mail::deploy_event_email_content(&payload);
            if let Err(e) = mail::send_plain_email(smtp_cfg, to, &subject, &body).await {
                tracing::warn!(?e, %to, "notification email failed");
            }
            continue;
        }
        let (post_url, body) = match notification_post_body(&kind, &target_url, &payload, &name) {
            Ok(x) => x,
            Err(e) => {
                tracing::warn!(?e, %target_url, "notification body build failed");
                continue;
            }
        };

        let res = client
            .post(&post_url)
            .json(&body)
            .header("X-DeployWerk-Event", event)
            .header("X-DeployWerk-Integration", &name)
            .send()
            .await;

        if let Err(e) = res {
            tracing::warn!(?e, %target_url, "notification webhook failed");
        }
    }

    Ok(())
}

async fn list_endpoints(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<NotificationEndpointRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let rows: Vec<(Uuid, Uuid, String, String, String, String, bool, DateTime<Utc>)> =
        sqlx::query_as(
            r#"SELECT id, team_id, name, kind, target_url, events, enabled, created_at
               FROM notification_endpoints WHERE team_id = $1 ORDER BY created_at DESC"#,
        )
        .bind(team_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    let out = rows
        .into_iter()
        .map(
            |(id, tid, name, kind, target_url, events, enabled, created_at)| NotificationEndpointRow {
                id,
                team_id: tid,
                name,
                kind,
                target_url: Some(target_url),
                events,
                enabled,
                created_at,
            },
        )
        .collect();

    Ok(Json(out))
}

async fn create_endpoint(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<CreateEndpointBody>,
) -> Result<(axum::http::StatusCode, Json<NotificationEndpointRow>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let kind = body.kind.trim().to_string();
    if kind != "generic_http"
        && kind != "discord_webhook"
        && kind != "telegram"
        && kind != "email"
    {
        return Err(ApiError::BadRequest(
            "kind must be generic_http, discord_webhook, telegram, or email".into(),
        ));
    }
    let url = body.target_url.trim();
    if url.is_empty() {
        return Err(ApiError::BadRequest("invalid target_url".into()));
    }
    if kind == "telegram" {
        parse_telegram_target(url).ok_or_else(|| {
            ApiError::BadRequest(
                "telegram target_url must be CHAT_ID|https://api.telegram.org/bot…/sendMessage".into(),
            )
        })?;
    } else if kind == "email" {
        if !is_email_recipient(url) {
            return Err(ApiError::BadRequest(
                "email kind requires target_url to be a recipient address".into(),
            ));
        }
    } else if !url.starts_with("http") {
        return Err(ApiError::BadRequest("invalid target_url".into()));
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    let events = body
        .events
        .unwrap_or_else(|| "deploy_succeeded,deploy_failed,deploy_started".into());

    sqlx::query(
        r#"INSERT INTO notification_endpoints
           (id, team_id, name, kind, target_url, events, enabled, created_at)
           VALUES ($1, $2, $3, $4, $5, $6, TRUE, $7)"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(body.name.trim())
    .bind(&kind)
    .bind(url)
    .bind(&events)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(NotificationEndpointRow {
            id,
            team_id,
            name: body.name,
            kind,
            target_url: Some(url.to_string()),
            events,
            enabled: true,
            created_at: now,
        }),
    ))
}

async fn update_endpoint(
    State(state): State<Arc<AppState>>,
    Path((team_id, endpoint_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(body): Json<UpdateEndpointBody>,
) -> Result<Json<NotificationEndpointRow>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let row: Option<(String, String, String, String, bool, DateTime<Utc>)> = sqlx::query_as(
        "SELECT name, kind, target_url, events, enabled, created_at FROM notification_endpoints WHERE id = $1 AND team_id = $2",
    )
    .bind(endpoint_id)
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((mut name, mut kind, mut target_url, mut events, mut enabled, created_at)) = row else {
        return Err(ApiError::NotFound);
    };

    if let Some(n) = body.name {
        name = n;
    }
    if let Some(k) = body.kind {
        let k = k.trim().to_string();
        if k != "generic_http" && k != "discord_webhook" && k != "telegram" && k != "email" {
            return Err(ApiError::BadRequest("invalid kind".into()));
        }
        kind = k;
    }
    if let Some(u) = body.target_url {
        let u = u.trim();
        if u.is_empty() {
            return Err(ApiError::BadRequest("invalid target_url".into()));
        }
        if kind == "telegram" {
            parse_telegram_target(u).ok_or_else(|| {
                ApiError::BadRequest(
                    "telegram target_url must be CHAT_ID|https://api.telegram.org/bot…/sendMessage".into(),
                )
            })?;
        } else if kind == "email" {
            if !is_email_recipient(u) {
                return Err(ApiError::BadRequest(
                    "email kind requires target_url to be a recipient address".into(),
                ));
            }
        } else if !u.starts_with("http") {
            return Err(ApiError::BadRequest("invalid target_url".into()));
        }
        target_url = u.to_string();
    }
    if let Some(e) = body.events {
        events = e;
    }
    if let Some(en) = body.enabled {
        enabled = en;
    }

    sqlx::query(
        r#"UPDATE notification_endpoints SET name = $1, kind = $2, target_url = $3, events = $4, enabled = $5
           WHERE id = $6 AND team_id = $7"#,
    )
    .bind(&name)
    .bind(&kind)
    .bind(&target_url)
    .bind(&events)
    .bind(enabled)
    .bind(endpoint_id)
    .bind(team_id)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(NotificationEndpointRow {
        id: endpoint_id,
        team_id,
        name,
        kind,
        target_url: Some(target_url),
        events,
        enabled,
        created_at,
    }))
}

async fn delete_endpoint(
    State(state): State<Arc<AppState>>,
    Path((team_id, endpoint_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<axum::http::StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let r = sqlx::query("DELETE FROM notification_endpoints WHERE id = $1 AND team_id = $2")
        .bind(endpoint_id)
        .bind(team_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    if r.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn test_endpoint(
    State(state): State<Arc<AppState>>,
    Path((team_id, endpoint_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT kind, target_url, name FROM notification_endpoints WHERE id = $1 AND team_id = $2",
    )
    .bind(endpoint_id)
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((kind, target_url, name)) = row else {
        return Err(ApiError::NotFound);
    };

    let payload = json!({
        "event": "test",
        "message": "DeployWerk test delivery",
        "ts": Utc::now().to_rfc3339(),
    });

    if kind == "email" {
        let Some(ref smtp) = state.smtp_settings else {
            return Err(ApiError::BadRequest(
                "SMTP is not configured on this instance (set DEPLOYWERK_SMTP_HOST and DEPLOYWERK_SMTP_FROM)"
                    .into(),
            ));
        };
        let to = target_url.trim();
        if !is_email_recipient(to) {
            return Err(ApiError::BadRequest(
                "email kind requires target_url to be a recipient address".into(),
            ));
        }
        let subject = "DeployWerk notification test";
        let body_txt = format!(
            "DeployWerk test delivery at {}\n",
            Utc::now().to_rfc3339()
        );
        match mail::send_plain_email(smtp, to, subject, &body_txt).await {
            Ok(()) => {
                return Ok(Json(
                    json!({ "ok": true, "http_status": 200, "channel": "smtp" }),
                ));
            }
            Err(e) => {
                tracing::warn!(?e, "smtp test failed");
                return Ok(Json(
                    json!({ "ok": false, "http_status": 502, "channel": "smtp", "detail": e }),
                ));
            }
        }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|_| ApiError::Internal)?;

    let (post_url, body) = if kind == "discord_webhook" || kind == "telegram" {
        notification_post_body(&kind, &target_url, &payload, &name)?
    } else {
        (target_url.clone(), payload.clone())
    };

    let res = client
        .post(&post_url)
        .json(&body)
        .header("X-DeployWerk-Event", "test")
        .header("X-DeployWerk-Integration", &name)
        .send()
        .await
        .map_err(|_| ApiError::Internal)?;

    let ok = res.status().is_success();
    let status = res.status().as_u16();
    Ok(Json(json!({ "ok": ok, "http_status": status })))
}
