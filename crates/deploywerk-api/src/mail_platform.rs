//! Mail platform Phase 1: transactional send API (SMTP-backed) + schema scaffold.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::require_principal;
use crate::error::ApiError;
use crate::mail;
use crate::rbac::{require_team_member, require_team_mutator};
use crate::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/teams/{team_id}/mail/domains", get(list_mail_domains).post(create_mail_domain))
        .route(
            "/api/v1/teams/{team_id}/mail/domains/{domain_id}",
            delete(delete_mail_domain),
        )
        .route(
            "/api/v1/teams/{team_id}/mail/domains/{domain_id}/dns-check",
            get(mail_domain_dns_stub),
        )
        .route("/api/v1/teams/{team_id}/mail/send", post(send_mail))
        .route(
            "/api/v1/teams/{team_id}/mail/messages/{message_id}",
            get(get_message_status),
        )
}

fn mail_enabled() -> bool {
    std::env::var("DEPLOYWERK_MAIL_ENABLED")
        .ok()
        .map(|v| v == "true")
        .unwrap_or(false)
}

#[derive(Serialize)]
struct MailDomainRow {
    id: Uuid,
    domain: String,
    status: String,
    created_at: chrono::DateTime<Utc>,
}

#[derive(Deserialize)]
struct CreateMailDomainBody {
    domain: String,
}

async fn list_mail_domains(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<MailDomainRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let rows: Vec<(Uuid, String, String, chrono::DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT id, domain, status, created_at FROM mail_domains WHERE team_id = $1 ORDER BY domain"#,
    )
    .bind(team_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(|(id, domain, status, created_at)| MailDomainRow {
                id,
                domain,
                status,
                created_at,
            })
            .collect(),
    ))
}

async fn create_mail_domain(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<CreateMailDomainBody>,
) -> Result<(StatusCode, Json<MailDomainRow>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let d = body.domain.trim().to_lowercase();
    if d.is_empty() || d.len() > 255 || !d.contains('.') {
        return Err(ApiError::BadRequest("invalid domain".into()));
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO mail_domains (id, team_id, domain, status, created_at) VALUES ($1,$2,$3,'pending',$4)"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(&d)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref d) = e {
            if d.code().as_deref() == Some("23505") {
                return ApiError::Conflict("domain already registered for team".into());
            }
        }
        tracing::warn!(?e, "mail domain insert failed");
        ApiError::Internal
    })?;

    Ok((
        StatusCode::CREATED,
        Json(MailDomainRow {
            id,
            domain: d,
            status: "pending".into(),
            created_at: now,
        }),
    ))
}

async fn delete_mail_domain(
    State(state): State<Arc<AppState>>,
    Path((team_id, domain_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let n = sqlx::query("DELETE FROM mail_domains WHERE id = $1 AND team_id = $2")
        .bind(domain_id)
        .bind(team_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?
        .rows_affected();
    if n == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Placeholder until live DNS validation (spec/08 wizard).
async fn mail_domain_dns_stub(
    State(state): State<Arc<AppState>>,
    Path((team_id, domain_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let row: Option<(String,)> = sqlx::query_as("SELECT domain FROM mail_domains WHERE id = $1 AND team_id = $2")
        .bind(domain_id)
        .bind(team_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    let Some((domain,)) = row else {
        return Err(ApiError::NotFound);
    };

    Ok(Json(serde_json::json!({
        "domain": domain,
        "mx": { "status": "not_checked", "note": "Live DNS lookups are not implemented yet; use your DNS provider to set records per spec/08." },
        "spf": { "status": "not_checked" },
        "dkim": { "status": "not_checked" },
        "dmarc": { "status": "not_checked" },
    })))
}

#[derive(Deserialize)]
struct SendMailBody {
    from: String,
    to: Vec<String>,
    subject: String,
    text: String,
}

#[derive(Serialize)]
struct SendMailResponse {
    message_id: Uuid,
    status: String,
}

async fn send_mail(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<SendMailBody>,
) -> Result<(StatusCode, Json<SendMailResponse>), ApiError> {
    if !mail_enabled() {
        return Err(ApiError::Forbidden);
    }
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_mutator(&state.pool, p.user_id, team_id).await?;

    let from = body.from.trim();
    if from.is_empty() || !from.contains('@') {
        return Err(ApiError::BadRequest("invalid from".into()));
    }
    if body.to.is_empty() || body.to.iter().any(|t| !t.contains('@')) {
        return Err(ApiError::BadRequest("invalid to".into()));
    }

    let subject = body.subject.trim();
    let text = body.text.trim();
    if text.is_empty() {
        return Err(ApiError::BadRequest("text required".into()));
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    let to_json = serde_json::to_value(&body.to).unwrap_or_else(|_| serde_json::json!([]));

    sqlx::query(
        r#"INSERT INTO mail_messages (id, team_id, from_addr, to_addrs, subject, text_body, status, created_at)
           VALUES ($1,$2,$3,$4,$5,$6,'queued',$7)"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(from)
    .bind(to_json)
    .bind(subject)
    .bind(text)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let smtp = state.smtp_settings.clone();
    if smtp.is_none() {
        sqlx::query("UPDATE mail_messages SET status = 'error', error_message = $1 WHERE id = $2")
            .bind("SMTP not configured on instance")
            .bind(id)
            .execute(&state.pool)
            .await
            .ok();
        return Ok((
            StatusCode::ACCEPTED,
            Json(SendMailResponse {
                message_id: id,
                status: "error".into(),
            }),
        ));
    }

    // Send one message per recipient for simplicity (Phase 1).
    let mut any_err: Option<String> = None;
    for to in &body.to {
        if let Err(e) = mail::send_plain_email(
            smtp.as_ref().unwrap(),
            to,
            subject,
            text,
        )
        .await
        {
            any_err = Some(format!("{e:?}"));
            break;
        }
    }

    if let Some(err_msg) = any_err {
        sqlx::query("UPDATE mail_messages SET status = 'error', error_message = $1 WHERE id = $2")
            .bind(err_msg)
            .bind(id)
            .execute(&state.pool)
            .await
            .ok();
        return Ok((
            StatusCode::ACCEPTED,
            Json(SendMailResponse {
                message_id: id,
                status: "error".into(),
            }),
        ));
    }

    sqlx::query("UPDATE mail_messages SET status = 'sent', sent_at = $1 WHERE id = $2")
        .bind(Utc::now())
        .bind(id)
        .execute(&state.pool)
        .await
        .ok();

    Ok((
        StatusCode::ACCEPTED,
        Json(SendMailResponse {
            message_id: id,
            status: "sent".into(),
        }),
    ))
}

#[derive(Serialize)]
struct MessageStatusRow {
    id: Uuid,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_message: Option<String>,
    created_at: chrono::DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sent_at: Option<chrono::DateTime<Utc>>,
}

async fn get_message_status(
    State(state): State<Arc<AppState>>,
    Path((team_id, message_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<MessageStatusRow>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let row: Option<(String, Option<String>, chrono::DateTime<Utc>, Option<chrono::DateTime<Utc>>)> =
        sqlx::query_as(
            "SELECT status, error_message, created_at, sent_at FROM mail_messages WHERE id = $1 AND team_id = $2",
        )
        .bind(message_id)
        .bind(team_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    let Some((status, error_message, created_at, sent_at)) = row else {
        return Err(ApiError::NotFound);
    };
    Ok(Json(MessageStatusRow {
        id: message_id,
        status,
        error_message,
        created_at,
        sent_at,
    }))
}

