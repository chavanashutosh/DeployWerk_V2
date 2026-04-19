use deploywerk_core::{AppRole, ApplicationMembershipSummary, TeamRole, UserSummary};
use crate::DbPool;
use uuid::Uuid;

use crate::auth::role_from_db;
use crate::error::ApiError;

pub async fn user_summary_with_rbac(pool: &DbPool, mut u: UserSummary) -> Result<UserSummary, ApiError> {
    u.organization_admin_organization_ids = organization_admin_ids_for_user(pool, u.id).await?;
    u.application_memberships = application_memberships_for_user(pool, u.id).await?;
    Ok(u)
}

pub async fn membership_role(
    pool: &DbPool,
    user_id: Uuid,
    team_id: Uuid,
) -> Result<Option<TeamRole>, ApiError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT role FROM team_memberships WHERE user_id = $1 AND team_id = $2")
            .bind(user_id)
            .bind(team_id)
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::Internal)?;
    Ok(row.map(|(r,)| role_from_db(&r)))
}

pub async fn organization_membership_role(
    pool: &DbPool,
    user_id: Uuid,
    organization_id: Uuid,
) -> Result<Option<TeamRole>, ApiError> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT role FROM organization_memberships WHERE user_id = $1 AND organization_id = $2",
    )
    .bind(user_id)
    .bind(organization_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    Ok(row.map(|(r,)| role_from_db(&r)))
}

pub async fn team_organization_id(pool: &DbPool, team_id: Uuid) -> Result<Uuid, ApiError> {
    let oid: Option<Uuid> = sqlx::query_scalar("SELECT organization_id FROM teams WHERE id = $1")
        .bind(team_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    oid.ok_or(ApiError::NotFound)
}

pub fn can_read_team(role: TeamRole) -> bool {
    matches!(
        role,
        TeamRole::Owner | TeamRole::Admin | TeamRole::Member
    )
}

pub fn can_mutate_team(role: TeamRole) -> bool {
    matches!(role, TeamRole::Owner | TeamRole::Admin)
}

pub fn can_read_org(role: TeamRole) -> bool {
    can_read_team(role)
}

pub fn can_mutate_org(role: TeamRole) -> bool {
    can_mutate_team(role)
}

pub async fn require_org_member(
    pool: &DbPool,
    user_id: Uuid,
    organization_id: Uuid,
) -> Result<TeamRole, ApiError> {
    let r = organization_membership_role(pool, user_id, organization_id)
        .await?
        .ok_or(ApiError::Forbidden)?;
    if !can_read_org(r) {
        return Err(ApiError::Forbidden);
    }
    Ok(r)
}

pub async fn require_org_mutator(
    pool: &DbPool,
    user_id: Uuid,
    organization_id: Uuid,
) -> Result<TeamRole, ApiError> {
    let r = require_org_member(pool, user_id, organization_id).await?;
    if !can_mutate_org(r) {
        return Err(ApiError::Forbidden);
    }
    Ok(r)
}

pub async fn require_org_owner(
    pool: &DbPool,
    user_id: Uuid,
    organization_id: Uuid,
) -> Result<(), ApiError> {
    let r = require_org_member(pool, user_id, organization_id).await?;
    if r != TeamRole::Owner {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

pub async fn require_team_member(
    pool: &DbPool,
    user_id: Uuid,
    team_id: Uuid,
) -> Result<TeamRole, ApiError> {
    let r = membership_role(pool, user_id, team_id)
        .await?
        .ok_or(ApiError::Forbidden)?;
    if !can_read_team(r) {
        return Err(ApiError::Forbidden);
    }
    let org_id = team_organization_id(pool, team_id).await?;
    organization_membership_role(pool, user_id, org_id)
        .await?
        .ok_or(ApiError::Forbidden)?;
    Ok(r)
}

pub async fn require_team_mutator(
    pool: &DbPool,
    user_id: Uuid,
    team_id: Uuid,
) -> Result<TeamRole, ApiError> {
    let r = require_team_member(pool, user_id, team_id).await?;
    if !can_mutate_team(r) {
        return Err(ApiError::Forbidden);
    }
    Ok(r)
}

pub async fn require_team_owner(
    pool: &DbPool,
    user_id: Uuid,
    team_id: Uuid,
) -> Result<(), ApiError> {
    let r = require_team_member(pool, user_id, team_id).await?;
    if r != TeamRole::Owner {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

pub fn app_role_from_db(s: &str) -> AppRole {
    AppRole::parse(s).unwrap_or(AppRole::Viewer)
}

pub async fn user_is_platform_admin(pool: &DbPool, user_id: Uuid) -> Result<bool, ApiError> {
    let v: Option<(bool,)> = sqlx::query_as("SELECT is_platform_admin FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    Ok(v.map(|(b,)| b).unwrap_or(false))
}

/// Orgs where the user is owner or admin (for org-admin UI / governance read paths).
pub async fn organization_admin_ids_for_user(pool: &DbPool, user_id: Uuid) -> Result<Vec<Uuid>, ApiError> {
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        r#"SELECT organization_id FROM organization_memberships WHERE user_id = $1 AND role IN ('owner', 'admin')"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

pub async fn application_memberships_for_user(
    pool: &DbPool,
    user_id: Uuid,
) -> Result<Vec<ApplicationMembershipSummary>, ApiError> {
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT application_id, role FROM application_memberships WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    Ok(rows
        .into_iter()
        .map(|(application_id, role)| ApplicationMembershipSummary {
            application_id,
            role: app_role_from_db(&role),
        })
        .collect())
}

pub async fn application_membership_role(
    pool: &DbPool,
    user_id: Uuid,
    application_id: Uuid,
) -> Result<Option<AppRole>, ApiError> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT role FROM application_memberships WHERE user_id = $1 AND application_id = $2",
    )
    .bind(user_id)
    .bind(application_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    Ok(row.map(|(r,)| app_role_from_db(&r)))
}

/// Resolve team and org for an application (path validation).
pub async fn application_team_and_org(
    pool: &DbPool,
    application_id: Uuid,
) -> Result<(Uuid, Uuid), ApiError> {
    let row: Option<(Uuid, Uuid)> = sqlx::query_as(
        r#"SELECT p.team_id, t.organization_id
           FROM applications a
           JOIN environments e ON e.id = a.environment_id
           JOIN projects p ON p.id = e.project_id
           JOIN teams t ON t.id = p.team_id
           WHERE a.id = $1"#,
    )
    .bind(application_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    row.ok_or(ApiError::NotFound)
}

/// True if application belongs to `team_id`.
pub async fn application_in_team(
    pool: &DbPool,
    application_id: Uuid,
    team_id: Uuid,
) -> Result<bool, ApiError> {
    let (tid, _) = application_team_and_org(pool, application_id).await?;
    Ok(tid == team_id)
}

/// Read access to team-scoped resources: team member, org owner/admin of parent org, or platform admin.
pub async fn require_team_access_read(pool: &DbPool, user_id: Uuid, team_id: Uuid) -> Result<(), ApiError> {
    if user_is_platform_admin(pool, user_id).await? {
        return Ok(());
    }
    if require_team_member(pool, user_id, team_id).await.is_ok() {
        return Ok(());
    }
    let org_id = team_organization_id(pool, team_id).await?;
    require_org_mutator(pool, user_id, org_id).await?;
    Ok(())
}

/// Write access to team resources (servers, invites, etc.): team owner/admin, org owner/admin, or platform admin.
pub async fn require_team_access_mutate(pool: &DbPool, user_id: Uuid, team_id: Uuid) -> Result<(), ApiError> {
    if user_is_platform_admin(pool, user_id).await? {
        return Ok(());
    }
    if membership_role(pool, user_id, team_id).await?.is_some() {
        require_team_mutator(pool, user_id, team_id).await?;
        return Ok(());
    }
    let org_id = team_organization_id(pool, team_id).await?;
    require_org_mutator(pool, user_id, org_id).await?;
    Ok(())
}

/// Read a specific application under a team path: team/org read, explicit app role, or platform admin.
pub async fn require_application_read(
    pool: &DbPool,
    user_id: Uuid,
    team_id: Uuid,
    application_id: Uuid,
) -> Result<(), ApiError> {
    if !application_in_team(pool, application_id, team_id).await? {
        return Err(ApiError::NotFound);
    }
    if user_is_platform_admin(pool, user_id).await? {
        return Ok(());
    }
    if let Some(ar) = application_membership_role(pool, user_id, application_id).await? {
        let _ = ar;
        return Ok(());
    }
    require_team_access_read(pool, user_id, team_id).await
}

/// Mutate app settings, env, delete: team mutator, app admin, or platform admin (not org-only, not app viewer).
pub async fn require_application_mutate(
    pool: &DbPool,
    user_id: Uuid,
    team_id: Uuid,
    application_id: Uuid,
) -> Result<(), ApiError> {
    if !application_in_team(pool, application_id, team_id).await? {
        return Err(ApiError::NotFound);
    }
    if user_is_platform_admin(pool, user_id).await? {
        return Ok(());
    }
    if let Some(AppRole::Admin) = application_membership_role(pool, user_id, application_id).await? {
        return Ok(());
    }
    require_team_mutator(pool, user_id, team_id)
        .await
        .map(|_| ())
}

/// App-only users may list apps in an environment if they have any membership here.
pub async fn require_some_app_membership_in_environment(
    pool: &DbPool,
    user_id: Uuid,
    environment_id: Uuid,
) -> Result<(), ApiError> {
    let n: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(1) FROM application_memberships am
           JOIN applications a ON a.id = am.application_id
           WHERE am.user_id = $1 AND a.environment_id = $2"#,
    )
    .bind(user_id)
    .bind(environment_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    if n > 0 {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}

pub async fn application_ids_for_user_in_environment(
    pool: &DbPool,
    user_id: Uuid,
    environment_id: Uuid,
) -> Result<Vec<Uuid>, ApiError> {
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        r#"SELECT am.application_id FROM application_memberships am
           JOIN applications a ON a.id = am.application_id
           WHERE am.user_id = $1 AND a.environment_id = $2"#,
    )
    .bind(user_id)
    .bind(environment_id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Env var secrets visible to platform admin, team owner/admin, or app admin (not app viewer, not org-only).
pub async fn user_can_see_application_secrets(
    pool: &DbPool,
    user_id: Uuid,
    team_id: Uuid,
    application_id: Uuid,
) -> Result<bool, ApiError> {
    if user_is_platform_admin(pool, user_id).await? {
        return Ok(true);
    }
    if let Some(AppRole::Admin) = application_membership_role(pool, user_id, application_id).await? {
        return Ok(true);
    }
    if let Ok(Some(r)) = membership_role(pool, user_id, team_id).await {
        if matches!(r, TeamRole::Owner | TeamRole::Admin) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Deploy / rollback: team member (existing behavior), team mutator, app admin, or platform admin. Not app-viewer-only, not org-only.
pub async fn require_application_deploy(
    pool: &DbPool,
    user_id: Uuid,
    team_id: Uuid,
    application_id: Uuid,
) -> Result<(), ApiError> {
    if !application_in_team(pool, application_id, team_id).await? {
        return Err(ApiError::NotFound);
    }
    if user_is_platform_admin(pool, user_id).await? {
        return Ok(());
    }
    if let Some(AppRole::Admin) = application_membership_role(pool, user_id, application_id).await? {
        return Ok(());
    }
    require_team_member(pool, user_id, team_id).await?;
    Ok(())
}
