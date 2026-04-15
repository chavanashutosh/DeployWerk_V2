//! Team-scoped deployment destinations on servers (Docker standalone).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use deploywerk_core::{DestinationKind, DestinationSummary};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::require_principal;
use crate::error::ApiError;
use crate::rbac::{require_team_access_mutate, require_team_member};
use crate::AppState;

/// Ensures each team has a built-in platform destination when the feature is enabled.
pub(crate) async fn ensure_platform_destination(
    pool: &sqlx::PgPool,
    team_id: Uuid,
    enabled: bool,
) -> Result<(), sqlx::Error> {
    if !enabled {
        return Ok(());
    }
    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO destinations (id, team_id, server_id, name, slug, kind, description, created_at)
           VALUES ($1, $2, NULL, $3, $4, 'docker_platform', $5, $6)
           ON CONFLICT (team_id, slug) DO NOTHING"#,
    )
    .bind(id)
    .bind(team_id)
    .bind("Platform (API host)")
    .bind("platform")
    .bind(Some(
        "Docker on the same machine as DeployWerk. Requires Docker on the API host and DEPLOYWERK_PLATFORM_DOCKER_ENABLED."
            .to_string(),
    ))
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/v1/teams/{team_id}/destinations",
            get(list_destinations).post(create_destination),
        )
        .route(
            "/api/v1/teams/{team_id}/destinations/{destination_id}",
            get(get_destination)
                .patch(update_destination)
                .delete(delete_destination),
        )
}

fn kind_from_db(s: &str) -> Result<DestinationKind, ApiError> {
    DestinationKind::parse(s).ok_or(ApiError::Internal)
}

async fn ensure_server_on_team(
    pool: &sqlx::PgPool,
    team_id: Uuid,
    server_id: Uuid,
) -> Result<(), ApiError> {
    let ok: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM servers WHERE id = $1 AND team_id = $2)",
    )
    .bind(server_id)
    .bind(team_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    if ok {
        Ok(())
    } else {
        Err(ApiError::BadRequest("server not found on this team".into()))
    }
}

async fn list_destinations(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<DestinationSummary>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let _ = ensure_platform_destination(
        &state.pool,
        team_id,
        state.deploy_worker.platform_docker_enabled,
    )
    .await;

    let rows: Vec<(
        Uuid,
        Uuid,
        Option<Uuid>,
        String,
        String,
        String,
        Option<String>,
        chrono::DateTime<Utc>,
    )> = sqlx::query_as(
        r#"SELECT id, team_id, server_id, name, slug, kind, description, created_at
           FROM destinations WHERE team_id = $1 ORDER BY name"#,
    )
    .bind(team_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let mut out = Vec::new();
    for (id, tid, server_id, name, slug, kind_s, description, created_at) in rows {
        out.push(DestinationSummary {
            id,
            team_id: tid,
            server_id,
            name,
            slug,
            kind: kind_from_db(&kind_s)?,
            description,
            created_at,
        });
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
struct CreateDestinationBody {
    #[serde(default)]
    server_id: Option<Uuid>,
    name: String,
    slug: String,
    #[serde(default = "default_kind")]
    kind: String,
    #[serde(default)]
    description: Option<String>,
}

fn default_kind() -> String {
    "docker_standalone".into()
}

async fn create_destination(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<CreateDestinationBody>,
) -> Result<(StatusCode, Json<DestinationSummary>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;

    let name = body.name.trim();
    let slug = body.slug.trim().to_lowercase();
    if name.is_empty() || slug.is_empty() {
        return Err(ApiError::BadRequest("name and slug required".into()));
    }
    let kind = DestinationKind::parse(body.kind.trim()).ok_or_else(|| {
        ApiError::BadRequest("kind must be docker_standalone or docker_platform".into())
    })?;

    let server_id = match kind {
        DestinationKind::DockerStandalone => {
            let sid = body
                .server_id
                .ok_or_else(|| ApiError::BadRequest("server_id required for docker_standalone".into()))?;
            ensure_server_on_team(&state.pool, team_id, sid).await?;
            Some(sid)
        }
        DestinationKind::DockerPlatform => {
            if !state.deploy_worker.platform_docker_enabled {
                return Err(ApiError::BadRequest(
                    "docker_platform requires DEPLOYWERK_PLATFORM_DOCKER_ENABLED".into(),
                ));
            }
            if body.server_id.is_some() {
                return Err(ApiError::BadRequest(
                    "docker_platform must not include server_id".into(),
                ));
            }
            None
        }
    };

    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"INSERT INTO destinations (id, team_id, server_id, name, slug, kind, description, created_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(server_id)
    .bind(name)
    .bind(&slug)
    .bind(kind.as_str())
    .bind(body.description.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()))
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("unique") || msg.contains("duplicate") {
            ApiError::Conflict("slug already exists on this team".into())
        } else {
            ApiError::Internal
        }
    })?;

    Ok((
        StatusCode::CREATED,
        Json(DestinationSummary {
            id,
            team_id,
            server_id,
            name: name.to_string(),
            slug,
            kind,
            description: body.description,
            created_at: now,
        }),
    ))
}

async fn get_destination(
    State(state): State<Arc<AppState>>,
    Path((team_id, destination_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<DestinationSummary>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_member(&state.pool, p.user_id, team_id).await?;

    let row: Option<(
        Uuid,
        Uuid,
        Option<Uuid>,
        String,
        String,
        String,
        Option<String>,
        chrono::DateTime<Utc>,
    )> = sqlx::query_as(
        r#"SELECT id, team_id, server_id, name, slug, kind, description, created_at
           FROM destinations WHERE id = $1 AND team_id = $2"#,
    )
    .bind(destination_id)
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((id, tid, server_id, name, slug, kind_s, description, created_at)) = row else {
        return Err(ApiError::NotFound);
    };

    Ok(Json(DestinationSummary {
        id,
        team_id: tid,
        server_id,
        name,
        slug,
        kind: kind_from_db(&kind_s)?,
        description,
        created_at,
    }))
}

#[derive(Deserialize)]
struct UpdateDestinationBody {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    server_id: Option<Uuid>,
    #[serde(default)]
    description: Option<Option<String>>,
}

async fn update_destination(
    State(state): State<Arc<AppState>>,
    Path((team_id, destination_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(body): Json<UpdateDestinationBody>,
) -> Result<Json<DestinationSummary>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;

    let row: Option<(
        Uuid,
        Uuid,
        Option<Uuid>,
        String,
        String,
        String,
        Option<String>,
        chrono::DateTime<Utc>,
    )> = sqlx::query_as(
        r#"SELECT id, team_id, server_id, name, slug, kind, description, created_at
           FROM destinations WHERE id = $1 AND team_id = $2"#,
    )
    .bind(destination_id)
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((id, tid, mut server_id, mut name, slug, kind_s, mut description, created_at)) = row
    else {
        return Err(ApiError::NotFound);
    };

    let dest_kind = kind_from_db(&kind_s)?;

    if let Some(sid) = body.server_id {
        if dest_kind == DestinationKind::DockerPlatform {
            return Err(ApiError::BadRequest(
                "cannot set server_id on docker_platform destination".into(),
            ));
        }
        ensure_server_on_team(&state.pool, team_id, sid).await?;
        server_id = Some(sid);
    }
    if let Some(n) = body.name.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        name = n.to_string();
    }
    if let Some(desc) = body.description {
        description = desc.and_then(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        });
    }

    sqlx::query(
        r#"UPDATE destinations SET name = $1, server_id = $2, description = $3
           WHERE id = $4 AND team_id = $5"#,
    )
    .bind(&name)
    .bind(server_id)
    .bind(&description)
    .bind(destination_id)
    .bind(team_id)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(DestinationSummary {
        id,
        team_id: tid,
        server_id,
        name,
        slug,
        kind: kind_from_db(&kind_s)?,
        description,
        created_at,
    }))
}

async fn delete_destination(
    State(state): State<Arc<AppState>>,
    Path((team_id, destination_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;

    let n = sqlx::query("DELETE FROM destinations WHERE id = $1 AND team_id = $2")
        .bind(destination_id)
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
