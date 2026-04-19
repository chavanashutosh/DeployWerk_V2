//! Team-level audit log helpers.

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::error::ApiError;

pub async fn log_team_audit(
    pool: &crate::DbPool,
    team_id: Uuid,
    actor_user_id: Uuid,
    action: &str,
    entity_type: &str,
    entity_id: Option<Uuid>,
    metadata: serde_json::Value,
    source_ip: Option<String>,
) -> Result<(), ApiError> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO team_audit_log
           (id, team_id, actor_user_id, action, entity_type, entity_id, metadata, source_ip, created_at)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(actor_user_id)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(metadata)
    .bind(source_ip)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    Ok(())
}

/// Best-effort audit log write (does not fail the request).
pub async fn try_log_team_audit(
    pool: &crate::DbPool,
    team_id: Uuid,
    actor_user_id: Uuid,
    action: &str,
    entity_type: &str,
    entity_id: Option<Uuid>,
    metadata: serde_json::Value,
    source_ip: Option<String>,
) {
    if let Err(e) = log_team_audit(
        pool,
        team_id,
        actor_user_id,
        action,
        entity_type,
        entity_id,
        metadata,
        source_ip,
    )
    .await
    {
        tracing::warn!(?e, team_id = %team_id, action, entity_type, "team audit write failed");
    }
}

#[derive(Serialize)]
pub struct TeamAuditRow {
    pub id: Uuid,
    pub actor_user_id: Uuid,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<Uuid>,
    pub metadata: serde_json::Value,
    pub source_ip: Option<String>,
    pub created_at: DateTime<Utc>,
}

