//! Platform product entitlements (per team). Distinct from customer `feature_flags`.

use chrono::Utc;
use crate::DbPool;
use uuid::Uuid;

use crate::error::ApiError;

/// Effective feature access: explicit `team_entitlements` row overrides `platform_feature_definitions.default_on`.
pub async fn team_has_feature(
    pool: &DbPool,
    team_id: Uuid,
    feature_key: &str,
) -> Result<bool, ApiError> {
    let def: Option<(bool,)> = sqlx::query_as(
        "SELECT default_on FROM platform_feature_definitions WHERE feature_key = $1",
    )
    .bind(feature_key)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((default_on,)) = def else {
        return Ok(false);
    };

    let row: Option<(bool, Option<chrono::DateTime<Utc>>)> = sqlx::query_as(
        "SELECT enabled, expires_at FROM team_entitlements WHERE team_id = $1 AND feature_key = $2",
    )
    .bind(team_id)
    .bind(feature_key)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    if let Some((enabled, exp)) = row {
        if let Some(t) = exp {
            if Utc::now() > t {
                return Ok(false);
            }
        }
        return Ok(enabled);
    }

    Ok(default_on)
}

pub async fn require_team_feature(
    pool: &DbPool,
    team_id: Uuid,
    feature_key: &'static str,
) -> Result<(), ApiError> {
    if team_has_feature(pool, team_id, feature_key).await? {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}
