//! Organization CRUD, members, and teams under an org.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use chrono::Utc;
use deploywerk_core::{OrganizationSummary, TeamRole, TeamSummary};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{require_principal, role_from_db};
use crate::error::ApiError;
use crate::slug::slugify;
use crate::rbac::{require_org_member, require_org_mutator, require_org_owner};
use crate::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/organizations", get(list_organizations).post(create_organization))
        .route(
            "/api/v1/organizations/{org_id}",
            get(get_organization)
                .patch(patch_organization)
                .delete(delete_organization),
        )
        .route(
            "/api/v1/organizations/{org_id}/members",
            get(list_organization_members),
        )
        .route(
            "/api/v1/organizations/{org_id}/members/{user_id}",
            patch(patch_organization_member).delete(delete_organization_member),
        )
        .route(
            "/api/v1/organizations/{org_id}/transfer-owner",
            post(transfer_organization_owner),
        )
        .route(
            "/api/v1/organizations/{org_id}/teams",
            get(list_organization_teams).post(create_team_in_organization),
        )
}

#[derive(Serialize)]
struct OrgMemberRow {
    user_id: Uuid,
    email: String,
    name: Option<String>,
    role: TeamRole,
}

async fn list_organizations(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<OrganizationSummary>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;

    let rows: Vec<(Uuid, String, String, String, bool)> = sqlx::query_as(
        r#"
        SELECT o.id, o.name, o.slug, m.role, o.mfa_required
        FROM organizations o
        JOIN organization_memberships m ON m.organization_id = o.id
        WHERE m.user_id = $1
        ORDER BY o.name
        "#,
    )
    .bind(p.user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let mut out = Vec::with_capacity(rows.len());
    for (id, name, slug, role_s, mfa_required) in rows {
        out.push(OrganizationSummary {
            id,
            name,
            slug,
            role: crate::auth::role_from_db(&role_s),
            mfa_required,
        });
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
struct CreateOrganizationBody {
    name: String,
    #[serde(default)]
    slug: Option<String>,
}

async fn create_organization(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateOrganizationBody>,
) -> Result<(StatusCode, Json<OrganizationSummary>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;

    let name = body.name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest("name required".into()));
    }
    let slug = body
        .slug
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| slugify(name));
    if slug.is_empty() {
        return Err(ApiError::BadRequest("invalid slug".into()));
    }

    let dup: i64 = sqlx::query_scalar("SELECT COUNT(1) FROM organizations WHERE slug = $1")
        .bind(&slug)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    if dup > 0 {
        return Err(ApiError::Conflict("organization slug already taken".into()));
    }

    let id = Uuid::new_v4();
    let now = Utc::now();

    let mut tx = state.pool.begin().await.map_err(|_| ApiError::Internal)?;
    sqlx::query("INSERT INTO organizations (id, name, slug, created_at) VALUES ($1, $2, $3, $4)")
        .bind(id)
        .bind(name)
        .bind(&slug)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?;

    sqlx::query(
        "INSERT INTO organization_memberships (user_id, organization_id, role) VALUES ($1, $2, 'owner')",
    )
    .bind(p.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;

    tx.commit().await.map_err(|_| ApiError::Internal)?;

    Ok((
        StatusCode::CREATED,
        Json(OrganizationSummary {
            id,
            name: name.to_string(),
            slug,
            role: TeamRole::Owner,
            mfa_required: false,
        }),
    ))
}

async fn get_organization(
    State(state): State<Arc<AppState>>,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<OrganizationSummary>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    let role = require_org_member(&state.pool, p.user_id, org_id).await?;

    let row: Option<(String, String, bool)> =
        sqlx::query_as("SELECT name, slug, mfa_required FROM organizations WHERE id = $1")
        .bind(org_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    let Some((name, slug, mfa_required)) = row else {
        return Err(ApiError::NotFound);
    };

    Ok(Json(OrganizationSummary {
        id: org_id,
        name,
        slug,
        role,
        mfa_required,
    }))
}

#[derive(Deserialize)]
struct PatchOrganizationBody {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    mfa_required: Option<bool>,
}

async fn patch_organization(
    State(state): State<Arc<AppState>>,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<PatchOrganizationBody>,
) -> Result<Json<OrganizationSummary>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;
    require_org_mutator(&state.pool, p.user_id, org_id).await?;

    if body.name.is_none() && body.slug.is_none() && body.mfa_required.is_none() {
        return Err(ApiError::BadRequest("no fields to update".into()));
    }

    let row: Option<(String, String, bool)> =
        sqlx::query_as("SELECT name, slug, mfa_required FROM organizations WHERE id = $1")
        .bind(org_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    let Some((cur_name, cur_slug, cur_mfa_required)) = row else {
        return Err(ApiError::NotFound);
    };

    let name = body
        .name
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(cur_name);
    let slug = if let Some(ref s) = body.slug {
        let t = s.trim();
        if t.is_empty() {
            return Err(ApiError::BadRequest("invalid slug".into()));
        }
        let t = t.to_string();
        let dup: i64 = sqlx::query_scalar(
            "SELECT COUNT(1) FROM organizations WHERE slug = $1 AND id <> $2",
        )
        .bind(&t)
        .bind(org_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
        if dup > 0 {
            return Err(ApiError::Conflict("organization slug already taken".into()));
        }
        t
    } else {
        cur_slug
    };

    let mfa_required = body.mfa_required.unwrap_or(cur_mfa_required);

    sqlx::query("UPDATE organizations SET name = $1, slug = $2, mfa_required = $3 WHERE id = $4")
        .bind(&name)
        .bind(&slug)
        .bind(mfa_required)
        .bind(org_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    let role = require_org_member(&state.pool, p.user_id, org_id).await?;
    Ok(Json(OrganizationSummary {
        id: org_id,
        name,
        slug,
        role,
        mfa_required,
    }))
}

async fn delete_organization(
    State(state): State<Arc<AppState>>,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;
    require_org_owner(&state.pool, p.user_id, org_id).await?;

    let n_teams: i64 = sqlx::query_scalar("SELECT COUNT(1)::bigint FROM teams WHERE organization_id = $1")
        .bind(org_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    if n_teams > 0 {
        return Err(ApiError::BadRequest("delete all teams in this organization before deleting the organization".into()));
    }

    let r = sqlx::query("DELETE FROM organizations WHERE id = $1")
        .bind(org_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    if r.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn list_organization_members(
    State(state): State<Arc<AppState>>,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<OrgMemberRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_org_member(&state.pool, p.user_id, org_id).await?;

    let rows: Vec<(Uuid, String, Option<String>, String)> = sqlx::query_as(
        r#"SELECT u.id, u.email, u.name, m.role
           FROM organization_memberships m
           JOIN users u ON u.id = m.user_id
           WHERE m.organization_id = $1
           ORDER BY u.email"#,
    )
    .bind(org_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(|(user_id, email, name, role_s)| OrgMemberRow {
                user_id,
                email,
                name,
                role: crate::auth::role_from_db(&role_s),
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct PatchOrganizationMemberBody {
    role: TeamRole,
}

async fn patch_organization_member(
    State(state): State<Arc<AppState>>,
    Path((org_id, user_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(body): Json<PatchOrganizationMemberBody>,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;
    require_org_mutator(&state.pool, p.user_id, org_id).await?;

    if body.role == TeamRole::Owner {
        return Err(ApiError::BadRequest(
            "use transfer-owner to assign organization owner".into(),
        ));
    }

    let cur_s: Option<String> = sqlx::query_scalar(
        "SELECT role FROM organization_memberships WHERE organization_id = $1 AND user_id = $2",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some(cur_s) = cur_s else {
        return Err(ApiError::NotFound);
    };

    if role_from_db(&cur_s) == TeamRole::Owner {
        return Err(ApiError::BadRequest(
            "cannot change role of organization owner; transfer ownership first".into(),
        ));
    }

    if role_from_db(&cur_s) == TeamRole::Admin && body.role == TeamRole::Member {
        let n_admins: i64 = sqlx::query_scalar(
            "SELECT COUNT(1)::bigint FROM organization_memberships WHERE organization_id = $1 AND role = 'admin'",
        )
        .bind(org_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
        if n_admins <= 1 {
            return Err(ApiError::BadRequest(
                "cannot demote the only organization admin".into(),
            ));
        }
    }

    let r = sqlx::query(
        "UPDATE organization_memberships SET role = $1 WHERE organization_id = $2 AND user_id = $3",
    )
    .bind(body.role.as_str())
    .bind(org_id)
    .bind(user_id)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    if r.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn delete_organization_member(
    State(state): State<Arc<AppState>>,
    Path((org_id, user_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;
    require_org_mutator(&state.pool, p.user_id, org_id).await?;

    let cur_s: Option<String> = sqlx::query_scalar(
        "SELECT role FROM organization_memberships WHERE organization_id = $1 AND user_id = $2",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some(cur_s) = cur_s else {
        return Err(ApiError::NotFound);
    };

    if role_from_db(&cur_s) == TeamRole::Owner {
        return Err(ApiError::BadRequest(
            "cannot remove organization owner; transfer ownership first".into(),
        ));
    }

    let still_in_org: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(1)::bigint FROM team_memberships tm JOIN teams t ON t.id = tm.team_id
           WHERE tm.user_id = $1 AND t.organization_id = $2"#,
    )
    .bind(user_id)
    .bind(org_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    if still_in_org > 0 {
        return Err(ApiError::BadRequest(
            "remove this user from all teams in this organization first".into(),
        ));
    }

    let r = sqlx::query(
        "DELETE FROM organization_memberships WHERE organization_id = $1 AND user_id = $2",
    )
    .bind(org_id)
    .bind(user_id)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    if r.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct TransferOrgOwnerBody {
    new_owner_user_id: Uuid,
}

async fn transfer_organization_owner(
    State(state): State<Arc<AppState>>,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<TransferOrgOwnerBody>,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;
    require_org_owner(&state.pool, p.user_id, org_id).await?;

    if body.new_owner_user_id == p.user_id {
        return Err(ApiError::BadRequest("already owner".into()));
    }

    let in_org: i64 = sqlx::query_scalar(
        "SELECT COUNT(1)::bigint FROM organization_memberships WHERE organization_id = $1 AND user_id = $2",
    )
    .bind(org_id)
    .bind(body.new_owner_user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    if in_org == 0 {
        return Err(ApiError::BadRequest("user is not a member of this organization".into()));
    }

    let mut tx = state.pool.begin().await.map_err(|_| ApiError::Internal)?;
    sqlx::query(
        "UPDATE organization_memberships SET role = 'admin' WHERE organization_id = $1 AND user_id = $2 AND role = 'owner'",
    )
    .bind(org_id)
    .bind(p.user_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;
    let r = sqlx::query(
        "UPDATE organization_memberships SET role = 'owner' WHERE organization_id = $1 AND user_id = $2",
    )
    .bind(org_id)
    .bind(body.new_owner_user_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;
    if r.rows_affected() != 1 {
        tx.rollback().await.ok();
        return Err(ApiError::Internal);
    }
    tx.commit().await.map_err(|_| ApiError::Internal)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn list_organization_teams(
    State(state): State<Arc<AppState>>,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamSummary>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    let org_role = require_org_member(&state.pool, p.user_id, org_id).await?;

    let rows: Vec<(Uuid, Uuid, String, String, String, bool)> = if crate::rbac::can_mutate_org(org_role) {
        sqlx::query_as(
            r#"
            SELECT t.id, t.organization_id, t.name, t.slug,
                   CASE WHEN m.team_id IS NOT NULL THEN m.role ELSE 'admin' END,
                   (m.team_id IS NOT NULL)
            FROM teams t
            LEFT JOIN team_memberships m ON m.team_id = t.id AND m.user_id = $2
            WHERE t.organization_id = $1
            ORDER BY t.name
            "#,
        )
        .bind(org_id)
        .bind(p.user_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?
    } else {
        sqlx::query_as(
            r#"
            SELECT t.id, t.organization_id, t.name, t.slug, m.role, true
            FROM teams t
            JOIN team_memberships m ON m.team_id = t.id
            WHERE t.organization_id = $1 AND m.user_id = $2
            ORDER BY t.name
            "#,
        )
        .bind(org_id)
        .bind(p.user_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?
    };

    let mut out = Vec::with_capacity(rows.len());
    for (tid, oid, name, slug, role_s, in_team) in rows {
        out.push(TeamSummary {
            id: tid,
            organization_id: oid,
            name,
            slug,
            role: crate::auth::role_from_db(&role_s),
            access_via_organization_admin: !in_team,
        });
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
struct CreateTeamInOrgBody {
    name: String,
    #[serde(default)]
    slug: Option<String>,
}

async fn create_team_in_organization(
    State(state): State<Arc<AppState>>,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<CreateTeamInOrgBody>,
) -> Result<(StatusCode, Json<TeamSummary>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;
    require_org_mutator(&state.pool, p.user_id, org_id).await?;

    let name = body.name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest("name required".into()));
    }
    let slug = body
        .slug
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| slugify(name));
    if slug.is_empty() {
        return Err(ApiError::BadRequest("invalid slug".into()));
    }

    let dup: i64 = sqlx::query_scalar(
        "SELECT COUNT(1) FROM teams WHERE organization_id = $1 AND slug = $2",
    )
    .bind(org_id)
    .bind(&slug)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    if dup > 0 {
        return Err(ApiError::Conflict(
            "team slug already taken in this organization".into(),
        ));
    }

    let team_id = Uuid::new_v4();
    let now = Utc::now();

    let mut tx = state.pool.begin().await.map_err(|_| ApiError::Internal)?;
    sqlx::query(
        "INSERT INTO teams (id, organization_id, name, slug, created_at) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(team_id)
    .bind(org_id)
    .bind(name)
    .bind(&slug)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;

    sqlx::query(
        "INSERT INTO team_memberships (user_id, team_id, role) VALUES ($1, $2, 'owner')",
    )
    .bind(p.user_id)
    .bind(team_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;

    tx.commit().await.map_err(|_| ApiError::Internal)?;

    Ok((
        StatusCode::CREATED,
        Json(TeamSummary {
            id: team_id,
            organization_id: org_id,
            name: name.to_string(),
            slug,
            role: TeamRole::Owner,
            access_via_organization_admin: false,
        }),
    ))
}
