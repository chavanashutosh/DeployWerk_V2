//! Platform operator APIs (`/api/v1/admin/*`). Requires JWT and `users.is_platform_admin`.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::{get, patch};
use axum::{Json, Router};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::require_principal;
use crate::error::ApiError;
use crate::integrations;
use crate::mail;
use crate::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/admin/users", get(list_users))
        .route("/api/v1/admin/users/{id}", get(get_user))
        .route(
            "/api/v1/admin/users/{id}/platform-admin",
            patch(patch_user_platform_admin),
        )
        .route("/api/v1/admin/organizations", get(list_organizations))
        .route(
            "/api/v1/admin/organizations/{id}",
            get(get_organization),
        )
        .route("/api/v1/admin/teams", get(list_teams))
        .route("/api/v1/admin/teams/{id}", get(get_team))
        .route("/api/v1/admin/billing", get(list_billing))
        .route(
            "/api/v1/admin/billing/{team_id}",
            patch(patch_admin_billing),
        )
        .route(
            "/api/v1/admin/billing/{team_id}/events",
            get(list_billing_events),
        )
        .route("/api/v1/admin/features", get(list_feature_definitions))
        .route(
            "/api/v1/admin/teams/{team_id}/entitlements",
            get(list_team_entitlements).post(upsert_team_entitlement),
        )
        .route("/api/v1/admin/analytics/overview", get(analytics_overview))
        .route("/api/v1/admin/system", get(admin_system))
        .route("/api/v1/admin/audit-log", get(list_audit_log))
        .route(
            "/api/v1/admin/integrations/portainer/health",
            get(admin_portainer_health),
        )
        .route(
            "/api/v1/admin/integrations/technitium/status",
            get(admin_technitium_status),
        )
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(default)]
    q: Option<String>,
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    50
}

fn clamp_limit(l: i64) -> i64 {
    l.clamp(1, 200)
}

async fn require_platform_admin(state: &AppState, headers: &HeaderMap) -> Result<Uuid, ApiError> {
    let p = require_principal(state, headers).await?;
    p.require_jwt()?;
    p.require_read()?;
    let ok: Option<(bool,)> = sqlx::query_as("SELECT is_platform_admin FROM users WHERE id = $1")
        .bind(p.user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    match ok {
        Some((true,)) => Ok(p.user_id),
        _ => Err(ApiError::Forbidden),
    }
}

fn spawn_platform_admin_notice_email(state: Arc<AppState>, actor: Uuid, target_user: Uuid, granted: bool) {
    if !state.admin_action_emails_enabled {
        return;
    }
    let Some(smtp) = state.smtp_settings.clone() else {
        tracing::debug!("admin action email skipped: SMTP not configured");
        return;
    };
    let pool = state.pool.clone();
    tokio::spawn(async move {
        let target_email: Option<String> = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
            .bind(target_user)
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten();
        let Some(target_email) = target_email else {
            return;
        };
        let actor_email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
            .bind(actor)
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "(unknown actor)".into());
        let (subject, body) = mail::admin_platform_admin_notice(granted, &target_email, &actor_email);
        if let Err(e) = mail::send_plain_email(&smtp, &target_email, &subject, &body).await {
            tracing::warn!(?e, to = %target_email, "admin platform-admin notification email failed");
        }
    });
}

fn spawn_billing_change_notice_emails(
    state: Arc<AppState>,
    actor: Uuid,
    team_id: Uuid,
    plan_name: String,
    status: String,
) {
    if !state.admin_action_emails_enabled {
        return;
    }
    let Some(smtp) = state.smtp_settings.clone() else {
        tracing::debug!("admin action email skipped: SMTP not configured");
        return;
    };
    let pool = state.pool.clone();
    tokio::spawn(async move {
        let team_name: String = sqlx::query_scalar("SELECT name FROM teams WHERE id = $1")
            .bind(team_id)
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "Team".into());
        let actor_email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
            .bind(actor)
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "(unknown actor)".into());
        let owners: Vec<String> = sqlx::query_scalar(
            r#"SELECT u.email FROM team_memberships tm
               INNER JOIN users u ON u.id = tm.user_id
               WHERE tm.team_id = $1 AND tm.role = 'owner'"#,
        )
        .bind(team_id)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();
        if owners.is_empty() {
            tracing::debug!(%team_id, "billing notice: no team owners to email");
            return;
        }
        let (subject, body) = mail::admin_billing_notice(&team_name, &plan_name, &status, &actor_email);
        for to in owners {
            if let Err(e) = mail::send_plain_email(&smtp, &to, &subject, &body).await {
                tracing::warn!(?e, %to, "admin billing notification email failed");
            }
        }
    });
}

pub async fn log_admin_audit(
    pool: &sqlx::PgPool,
    actor: Uuid,
    action: &str,
    entity_type: &str,
    entity_id: Option<Uuid>,
    metadata: serde_json::Value,
) -> Result<(), ApiError> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO admin_audit_log (id, actor_user_id, action, entity_type, entity_id, metadata, created_at)
           VALUES ($1,$2,$3,$4,$5,$6,$7)"#,
    )
    .bind(id)
    .bind(actor)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(metadata)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    Ok(())
}

// --- Users ---

#[derive(Serialize)]
struct AdminUserRow {
    id: Uuid,
    email: String,
    name: Option<String>,
    created_at: DateTime<Utc>,
    is_platform_admin: bool,
}

async fn list_users(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<AdminUserRow>>, ApiError> {
    let _actor = require_platform_admin(&state, &headers).await?;
    let limit = clamp_limit(q.limit);
    let offset = q.offset.max(0);

    let search = q.q.as_deref().unwrap_or("").trim();
    let like = format!("%{search}%");

    let rows: Vec<(Uuid, String, Option<String>, DateTime<Utc>, bool)> = if search.is_empty() {
        sqlx::query_as(
            r#"SELECT id, email, name, created_at, is_platform_admin FROM users
               ORDER BY created_at DESC LIMIT $1 OFFSET $2"#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?
    } else {
        sqlx::query_as(
            r#"SELECT id, email, name, created_at, is_platform_admin FROM users
               WHERE email ILIKE $1 OR COALESCE(name, '') ILIKE $1
               ORDER BY created_at DESC LIMIT $2 OFFSET $3"#,
        )
        .bind(&like)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?
    };

    Ok(Json(
        rows.into_iter()
            .map(
                |(id, email, name, created_at, is_platform_admin)| AdminUserRow {
                    id,
                    email,
                    name,
                    created_at,
                    is_platform_admin,
                },
            )
            .collect(),
    ))
}

#[derive(Serialize)]
struct OrgMembershipRow {
    organization_id: Uuid,
    org_name: String,
    org_slug: String,
    role: String,
}

#[derive(Serialize)]
struct TeamMembershipRow {
    team_id: Uuid,
    team_name: String,
    team_slug: String,
    organization_id: Uuid,
    role: String,
}

#[derive(Serialize)]
struct AdminUserDetail {
    user: AdminUserRow,
    organization_memberships: Vec<OrgMembershipRow>,
    team_memberships: Vec<TeamMembershipRow>,
}

async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<AdminUserDetail>, ApiError> {
    let _actor = require_platform_admin(&state, &headers).await?;

    let row: Option<(Uuid, String, Option<String>, DateTime<Utc>, bool)> = sqlx::query_as(
        "SELECT id, email, name, created_at, is_platform_admin FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((uid, email, name, created_at, is_platform_admin)) = row else {
        return Err(ApiError::NotFound);
    };

    let orgs: Vec<(Uuid, String, String, String)> = sqlx::query_as(
        r#"SELECT o.id, o.name, o.slug, om.role
           FROM organization_memberships om
           JOIN organizations o ON o.id = om.organization_id
           WHERE om.user_id = $1 ORDER BY o.name"#,
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let teams: Vec<(Uuid, String, String, Uuid, String)> = sqlx::query_as(
        r#"SELECT t.id, t.name, t.slug, t.organization_id, tm.role
           FROM team_memberships tm
           JOIN teams t ON t.id = tm.team_id
           WHERE tm.user_id = $1 ORDER BY t.name"#,
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(AdminUserDetail {
        user: AdminUserRow {
            id: uid,
            email,
            name,
            created_at,
            is_platform_admin,
        },
        organization_memberships: orgs
            .into_iter()
            .map(|(organization_id, org_name, org_slug, role)| OrgMembershipRow {
                organization_id,
                org_name,
                org_slug,
                role,
            })
            .collect(),
        team_memberships: teams
            .into_iter()
            .map(|(team_id, team_name, team_slug, organization_id, role)| TeamMembershipRow {
                team_id,
                team_name,
                team_slug,
                organization_id,
                role,
            })
            .collect(),
    }))
}

#[derive(Deserialize)]
struct PatchPlatformAdminBody {
    is_platform_admin: bool,
}

async fn patch_user_platform_admin(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<PatchPlatformAdminBody>,
) -> Result<Json<AdminUserRow>, ApiError> {
    let actor = require_platform_admin(&state, &headers).await?;

    if id == actor && !body.is_platform_admin {
        return Err(ApiError::BadRequest(
            "cannot remove your own platform admin access".into(),
        ));
    }

    sqlx::query("UPDATE users SET is_platform_admin = $1 WHERE id = $2")
        .bind(body.is_platform_admin)
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    log_admin_audit(
        &state.pool,
        actor,
        "user.platform_admin",
        "user",
        Some(id),
        serde_json::json!({ "is_platform_admin": body.is_platform_admin }),
    )
    .await?;

    spawn_platform_admin_notice_email(state.clone(), actor, id, body.is_platform_admin);

    let row: (Uuid, String, Option<String>, DateTime<Utc>, bool) = sqlx::query_as(
        "SELECT id, email, name, created_at, is_platform_admin FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let (uid, email, name, created_at, is_platform_admin) = row;
    Ok(Json(AdminUserRow {
        id: uid,
        email,
        name,
        created_at,
        is_platform_admin,
    }))
}

// --- Organizations ---

#[derive(Serialize)]
struct AdminOrgRow {
    id: Uuid,
    name: String,
    slug: String,
    created_at: DateTime<Utc>,
    team_count: i64,
}

async fn list_organizations(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<AdminOrgRow>>, ApiError> {
    let _actor = require_platform_admin(&state, &headers).await?;
    let limit = clamp_limit(q.limit);
    let offset = q.offset.max(0);
    let search = q.q.as_deref().unwrap_or("").trim();
    let like = format!("%{search}%");

    let rows: Vec<(Uuid, String, String, DateTime<Utc>, i64)> = if search.is_empty() {
        sqlx::query_as(
            r#"SELECT o.id, o.name, o.slug, o.created_at,
                      (SELECT COUNT(*)::bigint FROM teams t WHERE t.organization_id = o.id) AS team_count
               FROM organizations o
               ORDER BY o.created_at DESC
               LIMIT $1 OFFSET $2"#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?
    } else {
        sqlx::query_as(
            r#"SELECT o.id, o.name, o.slug, o.created_at,
                      (SELECT COUNT(*)::bigint FROM teams t WHERE t.organization_id = o.id) AS team_count
               FROM organizations o
               WHERE o.name ILIKE $1 OR o.slug ILIKE $1
               ORDER BY o.created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(&like)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?
    };

    Ok(Json(
        rows.into_iter()
            .map(|(id, name, slug, created_at, team_count)| AdminOrgRow {
                id,
                name,
                slug,
                created_at,
                team_count,
            })
            .collect(),
    ))
}

#[derive(Serialize)]
struct OrgMemberRow {
    user_id: Uuid,
    email: String,
    name: Option<String>,
    role: String,
}

#[derive(Serialize)]
struct AdminOrgDetail {
    organization: AdminOrgRow,
    members: Vec<OrgMemberRow>,
    teams: Vec<AdminTeamBrief>,
}

#[derive(Serialize)]
struct AdminTeamBrief {
    id: Uuid,
    name: String,
    slug: String,
}

async fn get_organization(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<AdminOrgDetail>, ApiError> {
    let _actor = require_platform_admin(&state, &headers).await?;

    let row: Option<(Uuid, String, String, DateTime<Utc>, i64)> = sqlx::query_as(
        r#"SELECT o.id, o.name, o.slug, o.created_at,
                  (SELECT COUNT(*)::bigint FROM teams t WHERE t.organization_id = o.id) AS team_count
           FROM organizations o WHERE o.id = $1"#,
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((oid, name, slug, created_at, team_count)) = row else {
        return Err(ApiError::NotFound);
    };

    let members: Vec<(Uuid, String, Option<String>, String)> = sqlx::query_as(
        r#"SELECT u.id, u.email, u.name, om.role
           FROM organization_memberships om
           JOIN users u ON u.id = om.user_id
           WHERE om.organization_id = $1
           ORDER BY u.email"#,
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let teams: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT id, name, slug FROM teams WHERE organization_id = $1 ORDER BY name",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(AdminOrgDetail {
        organization: AdminOrgRow {
            id: oid,
            name,
            slug,
            created_at,
            team_count,
        },
        members: members
            .into_iter()
            .map(|(user_id, email, name, role)| OrgMemberRow {
                user_id,
                email,
                name,
                role,
            })
            .collect(),
        teams: teams
            .into_iter()
            .map(|(tid, name, slug)| AdminTeamBrief {
                id: tid,
                name,
                slug,
            })
            .collect(),
    }))
}

// --- Teams ---

#[derive(Serialize)]
struct AdminTeamRow {
    id: Uuid,
    organization_id: Uuid,
    org_name: String,
    name: String,
    slug: String,
    created_at: DateTime<Utc>,
}

async fn list_teams(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<AdminTeamRow>>, ApiError> {
    let _actor = require_platform_admin(&state, &headers).await?;
    let limit = clamp_limit(q.limit);
    let offset = q.offset.max(0);
    let search = q.q.as_deref().unwrap_or("").trim();
    let like = format!("%{search}%");

    let rows: Vec<(Uuid, Uuid, String, String, String, DateTime<Utc>)> = if search.is_empty() {
        sqlx::query_as(
            r#"SELECT t.id, t.organization_id, o.name, t.name, t.slug, t.created_at
               FROM teams t
               JOIN organizations o ON o.id = t.organization_id
               ORDER BY t.created_at DESC
               LIMIT $1 OFFSET $2"#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?
    } else {
        sqlx::query_as(
            r#"SELECT t.id, t.organization_id, o.name, t.name, t.slug, t.created_at
               FROM teams t
               JOIN organizations o ON o.id = t.organization_id
               WHERE t.name ILIKE $1 OR t.slug ILIKE $1 OR o.name ILIKE $1
               ORDER BY t.created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(&like)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?
    };

    Ok(Json(
        rows.into_iter()
            .map(
                |(id, organization_id, org_name, name, slug, created_at)| AdminTeamRow {
                    id,
                    organization_id,
                    org_name,
                    name,
                    slug,
                    created_at,
                },
            )
            .collect(),
    ))
}

#[derive(Serialize)]
struct TeamMemberAdminRow {
    user_id: Uuid,
    email: String,
    name: Option<String>,
    role: String,
}

#[derive(Serialize)]
struct AdminTeamDetail {
    team: AdminTeamRow,
    members: Vec<TeamMemberAdminRow>,
    billing: Option<AdminBillingRow>,
}

#[derive(Serialize)]
struct AdminBillingRow {
    team_id: Uuid,
    plan_name: String,
    status: String,
    payment_provider: String,
    provider_customer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stripe_customer_id: Option<String>,
    updated_at: DateTime<Utc>,
}

async fn get_team(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<AdminTeamDetail>, ApiError> {
    let _actor = require_platform_admin(&state, &headers).await?;

    let row: Option<(Uuid, Uuid, String, String, String, DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT t.id, t.organization_id, o.name, t.name, t.slug, t.created_at
           FROM teams t
           JOIN organizations o ON o.id = t.organization_id
           WHERE t.id = $1"#,
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((tid, organization_id, org_name, name, slug, created_at)) = row else {
        return Err(ApiError::NotFound);
    };

    let members: Vec<(Uuid, String, Option<String>, String)> = sqlx::query_as(
        r#"SELECT u.id, u.email, u.name, tm.role
           FROM team_memberships tm
           JOIN users u ON u.id = tm.user_id
           WHERE tm.team_id = $1
           ORDER BY u.email"#,
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let bill: Option<(
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        DateTime<Utc>,
    )> = sqlx::query_as(
        r#"SELECT plan_name, status, payment_provider, provider_customer_id, stripe_customer_id, updated_at
           FROM team_billing WHERE team_id = $1"#,
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let billing = bill.map(
        |(plan_name, status, payment_provider, provider_customer_id, stripe_customer_id, updated_at)| {
            AdminBillingRow {
                team_id: tid,
                plan_name,
                status,
                payment_provider,
                provider_customer_id,
                stripe_customer_id,
                updated_at,
            }
        },
    );

    Ok(Json(AdminTeamDetail {
        team: AdminTeamRow {
            id: tid,
            organization_id,
            org_name,
            name,
            slug,
            created_at,
        },
        members: members
            .into_iter()
            .map(|(user_id, email, name, role)| TeamMemberAdminRow {
                user_id,
                email,
                name,
                role,
            })
            .collect(),
        billing,
    }))
}

// --- Billing directory ---

#[derive(Serialize)]
struct AdminBillingListRow {
    team_id: Uuid,
    team_name: String,
    team_slug: String,
    organization_id: Uuid,
    org_name: String,
    plan_name: String,
    status: String,
    payment_provider: String,
    provider_customer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stripe_customer_id: Option<String>,
    updated_at: DateTime<Utc>,
}

async fn list_billing(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<AdminBillingListRow>>, ApiError> {
    let _actor = require_platform_admin(&state, &headers).await?;
    let limit = clamp_limit(q.limit);
    let offset = q.offset.max(0);
    let search = q.q.as_deref().unwrap_or("").trim();
    let like = format!("%{search}%");

    let rows: Vec<(
        Uuid,
        String,
        String,
        Uuid,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        DateTime<Utc>,
    )> = if search.is_empty() {
        sqlx::query_as(
            r#"SELECT t.id, t.name, t.slug, t.organization_id, o.name,
                      COALESCE(tb.plan_name, 'free'), COALESCE(tb.status, 'inactive'),
                      COALESCE(tb.payment_provider, 'none'),
                      tb.provider_customer_id, tb.stripe_customer_id,
                      COALESCE(tb.updated_at, t.created_at)
               FROM teams t
               JOIN organizations o ON o.id = t.organization_id
               LEFT JOIN team_billing tb ON tb.team_id = t.id
               ORDER BY COALESCE(tb.updated_at, t.created_at) DESC
               LIMIT $1 OFFSET $2"#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?
    } else {
        sqlx::query_as(
            r#"SELECT t.id, t.name, t.slug, t.organization_id, o.name,
                      COALESCE(tb.plan_name, 'free'), COALESCE(tb.status, 'inactive'),
                      COALESCE(tb.payment_provider, 'none'),
                      tb.provider_customer_id, tb.stripe_customer_id,
                      COALESCE(tb.updated_at, t.created_at)
               FROM teams t
               JOIN organizations o ON o.id = t.organization_id
               LEFT JOIN team_billing tb ON tb.team_id = t.id
               WHERE t.name ILIKE $1 OR t.slug ILIKE $1 OR o.name ILIKE $1
                  OR COALESCE(tb.plan_name,'') ILIKE $1
               ORDER BY COALESCE(tb.updated_at, t.created_at) DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(&like)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?
    };

    Ok(Json(
        rows.into_iter()
            .map(
                |(
                    team_id,
                    team_name,
                    team_slug,
                    organization_id,
                    org_name,
                    plan_name,
                    status,
                    payment_provider,
                    provider_customer_id,
                    stripe_customer_id,
                    updated_at,
                )| AdminBillingListRow {
                    team_id,
                    team_name,
                    team_slug,
                    organization_id,
                    org_name,
                    plan_name,
                    status,
                    payment_provider,
                    provider_customer_id,
                    stripe_customer_id,
                    updated_at,
                },
            )
            .collect(),
    ))
}

#[derive(Deserialize, Serialize)]
struct PatchAdminBillingBody {
    plan_name: String,
    status: String,
    #[serde(default)]
    payment_provider: Option<String>,
    #[serde(default)]
    provider_customer_id: Option<String>,
    #[serde(default)]
    stripe_customer_id: Option<String>,
}

async fn patch_admin_billing(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<PatchAdminBillingBody>,
) -> Result<Json<AdminBillingRow>, ApiError> {
    let actor = require_platform_admin(&state, &headers).await?;

    let exists: i64 = sqlx::query_scalar("SELECT COUNT(1)::bigint FROM teams WHERE id = $1")
        .bind(team_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    if exists == 0 {
        return Err(ApiError::NotFound);
    }

    let payment_provider = body
        .payment_provider
        .clone()
        .unwrap_or_else(|| "none".into());
    let prov_id = body
        .provider_customer_id
        .clone()
        .or_else(|| body.stripe_customer_id.clone());

    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO team_billing (team_id, stripe_customer_id, plan_name, status, updated_at, payment_provider, provider_customer_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           ON CONFLICT (team_id) DO UPDATE SET
             stripe_customer_id = COALESCE(EXCLUDED.stripe_customer_id, team_billing.stripe_customer_id),
             plan_name = EXCLUDED.plan_name,
             status = EXCLUDED.status,
             updated_at = EXCLUDED.updated_at,
             payment_provider = EXCLUDED.payment_provider,
             provider_customer_id = COALESCE(EXCLUDED.provider_customer_id, team_billing.provider_customer_id)"#,
    )
    .bind(team_id)
    .bind(&body.stripe_customer_id)
    .bind(&body.plan_name)
    .bind(&body.status)
    .bind(now)
    .bind(&payment_provider)
    .bind(&prov_id)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    if body.stripe_customer_id.is_some() {
        sqlx::query(
            "UPDATE team_billing SET provider_customer_id = COALESCE(provider_customer_id, stripe_customer_id) WHERE team_id = $1",
        )
        .bind(team_id)
        .execute(&state.pool)
        .await
        .ok();
    }

    log_admin_audit(
        &state.pool,
        actor,
        "billing.patch",
        "team",
        Some(team_id),
        serde_json::to_value(&body).unwrap_or_default(),
    )
    .await?;

    spawn_billing_change_notice_emails(
        state.clone(),
        actor,
        team_id,
        body.plan_name.clone(),
        body.status.clone(),
    );

    let row: (String, String, String, Option<String>, Option<String>, DateTime<Utc>) = sqlx::query_as(
        r#"SELECT plan_name, status, payment_provider, provider_customer_id, stripe_customer_id, updated_at
           FROM team_billing WHERE team_id = $1"#,
    )
    .bind(team_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let (plan_name, status, payment_provider, provider_customer_id, stripe_customer_id, updated_at) =
        row;
    Ok(Json(AdminBillingRow {
        team_id,
        plan_name,
        status,
        payment_provider,
        provider_customer_id,
        stripe_customer_id,
        updated_at,
    }))
}

#[derive(Serialize)]
struct BillingEventRow {
    id: Uuid,
    team_id: Option<Uuid>,
    provider: String,
    event_code: String,
    psp_reference: Option<String>,
    merchant_reference: Option<String>,
    created_at: DateTime<Utc>,
}

async fn list_billing_events(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<BillingEventRow>>, ApiError> {
    let _actor = require_platform_admin(&state, &headers).await?;
    let limit = clamp_limit(q.limit);

    let rows: Vec<(Uuid, Option<Uuid>, String, String, Option<String>, Option<String>, DateTime<Utc>)> =
        sqlx::query_as(
            r#"SELECT id, team_id, provider, event_code, psp_reference, merchant_reference, created_at
               FROM billing_events WHERE team_id = $1
               ORDER BY created_at DESC LIMIT $2"#,
        )
        .bind(team_id)
        .bind(limit)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(
                |(
                    id,
                    team_id,
                    provider,
                    event_code,
                    psp_reference,
                    merchant_reference,
                    created_at,
                )| BillingEventRow {
                    id,
                    team_id,
                    provider,
                    event_code,
                    psp_reference,
                    merchant_reference,
                    created_at,
                },
            )
            .collect(),
    ))
}

// --- Feature definitions & entitlements ---

#[derive(Serialize)]
struct FeatureDefRow {
    feature_key: String,
    description: String,
    default_on: bool,
    created_at: DateTime<Utc>,
}

async fn list_feature_definitions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<FeatureDefRow>>, ApiError> {
    let _actor = require_platform_admin(&state, &headers).await?;
    let rows: Vec<(String, String, bool, DateTime<Utc>)> = sqlx::query_as(
        "SELECT feature_key, description, default_on, created_at FROM platform_feature_definitions ORDER BY feature_key",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(
                |(feature_key, description, default_on, created_at)| FeatureDefRow {
                    feature_key,
                    description,
                    default_on,
                    created_at,
                },
            )
            .collect(),
    ))
}

#[derive(Serialize)]
struct EntitlementRow {
    feature_key: String,
    enabled: bool,
    source: String,
    expires_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
    default_on: bool,
    effective: bool,
}

async fn list_team_entitlements(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<EntitlementRow>>, ApiError> {
    let _actor = require_platform_admin(&state, &headers).await?;

    let defs: Vec<(String, bool)> = sqlx::query_as(
        "SELECT feature_key, default_on FROM platform_feature_definitions ORDER BY feature_key",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let ents: Vec<(String, bool, String, Option<DateTime<Utc>>, DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT feature_key, enabled, source, expires_at, updated_at
           FROM team_entitlements WHERE team_id = $1"#,
    )
    .bind(team_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    use std::collections::HashMap;
    let mut m: HashMap<String, (bool, String, Option<DateTime<Utc>>, DateTime<Utc>)> =
        HashMap::new();
    for (k, en, src, exp, up) in ents {
        m.insert(k, (en, src, exp, up));
    }

    let mut out = Vec::new();
    for (feature_key, default_on) in defs {
        let effective = crate::entitlements::team_has_feature(&state.pool, team_id, &feature_key)
            .await?;
        if let Some((enabled, source, expires_at, updated_at)) = m.remove(&feature_key) {
            out.push(EntitlementRow {
                feature_key,
                enabled,
                source,
                expires_at,
                updated_at,
                default_on,
                effective,
            });
        } else {
            out.push(EntitlementRow {
                feature_key: feature_key.clone(),
                enabled: default_on,
                source: "default".into(),
                expires_at: None,
                updated_at: Utc::now(),
                default_on,
                effective,
            });
        }
    }

    Ok(Json(out))
}

#[derive(Deserialize)]
struct UpsertEntitlementBody {
    feature_key: String,
    enabled: bool,
    #[serde(default = "default_source_manual")]
    source: String,
    #[serde(default)]
    expires_at: Option<DateTime<Utc>>,
}

fn default_source_manual() -> String {
    "manual".into()
}

async fn upsert_team_entitlement(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<UpsertEntitlementBody>,
) -> Result<Json<EntitlementRow>, ApiError> {
    let actor = require_platform_admin(&state, &headers).await?;

    if !matches!(body.source.as_str(), "plan" | "manual" | "trial") {
        return Err(ApiError::BadRequest(
            "source must be plan, manual, or trial".into(),
        ));
    }

    let exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(1)::bigint FROM platform_feature_definitions WHERE feature_key = $1",
    )
    .bind(&body.feature_key)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    if exists == 0 {
        return Err(ApiError::BadRequest("unknown feature_key".into()));
    }

    let team_ok: i64 = sqlx::query_scalar("SELECT COUNT(1)::bigint FROM teams WHERE id = $1")
        .bind(team_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    if team_ok == 0 {
        return Err(ApiError::NotFound);
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO team_entitlements (id, team_id, feature_key, enabled, source, expires_at, created_at, updated_at)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
           ON CONFLICT (team_id, feature_key) DO UPDATE SET
             enabled = EXCLUDED.enabled,
             source = EXCLUDED.source,
             expires_at = EXCLUDED.expires_at,
             updated_at = EXCLUDED.updated_at"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(&body.feature_key)
    .bind(body.enabled)
    .bind(&body.source)
    .bind(body.expires_at)
    .bind(now)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let default_on: bool = sqlx::query_scalar(
        "SELECT default_on FROM platform_feature_definitions WHERE feature_key = $1",
    )
    .bind(&body.feature_key)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let effective =
        crate::entitlements::team_has_feature(&state.pool, team_id, &body.feature_key).await?;

    log_admin_audit(
        &state.pool,
        actor,
        "entitlement.upsert",
        "team",
        Some(team_id),
        serde_json::json!({
            "feature_key": body.feature_key,
            "enabled": body.enabled,
            "source": body.source,
            "expires_at": body.expires_at,
        }),
    )
    .await?;

    Ok(Json(EntitlementRow {
        feature_key: body.feature_key.clone(),
        enabled: body.enabled,
        source: body.source,
        expires_at: body.expires_at,
        updated_at: now,
        default_on,
        effective,
    }))
}

// --- Analytics ---

#[derive(Deserialize)]
struct DaysQuery {
    #[serde(default = "default_days")]
    days: i64,
}

fn default_days() -> i64 {
    30
}

#[derive(Serialize)]
struct AnalyticsOverview {
    days: i64,
    total_users: i64,
    total_organizations: i64,
    total_teams: i64,
    user_signups_by_day: Vec<(NaiveDate, i64)>,
    teams_created_by_day: Vec<(NaiveDate, i64)>,
    deploy_jobs_by_day: Vec<(NaiveDate, String, i64)>,
    rum_events_by_day: Vec<(NaiveDate, i64)>,
    billing_by_plan: Vec<(String, String, i64)>,
}

async fn analytics_overview(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<DaysQuery>,
) -> Result<Json<AnalyticsOverview>, ApiError> {
    let _actor = require_platform_admin(&state, &headers).await?;
    let days = q.days.clamp(1, 365);
    let since = Utc::now() - chrono::Duration::days(days);

    let total_users: i64 =
        sqlx::query_scalar("SELECT COUNT(1)::bigint FROM users")
            .fetch_one(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
    let total_organizations: i64 =
        sqlx::query_scalar("SELECT COUNT(1)::bigint FROM organizations")
            .fetch_one(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
    let total_teams: i64 = sqlx::query_scalar("SELECT COUNT(1)::bigint FROM teams")
        .fetch_one(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    let user_signups_by_day: Vec<(NaiveDate, i64)> = sqlx::query_as(
        r#"SELECT (created_at AT TIME ZONE 'UTC')::date AS d, COUNT(*)::bigint
           FROM users WHERE created_at >= $1
           GROUP BY 1 ORDER BY 1"#,
    )
    .bind(since)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let teams_created_by_day: Vec<(NaiveDate, i64)> = sqlx::query_as(
        r#"SELECT (created_at AT TIME ZONE 'UTC')::date AS d, COUNT(*)::bigint
           FROM teams WHERE created_at >= $1
           GROUP BY 1 ORDER BY 1"#,
    )
    .bind(since)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let deploy_jobs_by_day: Vec<(NaiveDate, String, i64)> = sqlx::query_as(
        r#"SELECT (dj.created_at AT TIME ZONE 'UTC')::date AS d, dj.status::text, COUNT(*)::bigint
           FROM deploy_jobs dj
           WHERE dj.created_at >= $1
           GROUP BY 1, 2 ORDER BY 1, 2"#,
    )
    .bind(since)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let rum_events_by_day: Vec<(NaiveDate, i64)> = sqlx::query_as(
        r#"SELECT (recorded_at AT TIME ZONE 'UTC')::date AS d, COUNT(*)::bigint
           FROM rum_events WHERE recorded_at >= $1
           GROUP BY 1 ORDER BY 1"#,
    )
    .bind(since)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let billing_by_plan: Vec<(String, String, i64)> = sqlx::query_as(
        r#"SELECT plan_name, status, COUNT(*)::bigint FROM team_billing GROUP BY 1, 2 ORDER BY 1, 2"#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(AnalyticsOverview {
        days,
        total_users,
        total_organizations,
        total_teams,
        user_signups_by_day,
        teams_created_by_day,
        deploy_jobs_by_day,
        rum_events_by_day,
        billing_by_plan,
    }))
}

// --- System ---

async fn admin_system(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _actor = require_platform_admin(&state, &headers).await?;

    let db_ok = sqlx::query_scalar::<_, i64>("SELECT 1::bigint")
        .fetch_one(&state.pool)
        .await
        .is_ok();

    let git_sha = state
        .deploywerk_git_sha
        .clone()
        .or_else(|| option_env!("DEPLOYWERK_GIT_SHA").map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown".into());

    Ok(Json(serde_json::json!({
        "database_ok": db_ok,
        "git_sha": git_sha,
    })))
}

// --- Optional operator integrations (Portainer / Technitium) ---

async fn admin_portainer_health(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let actor = require_platform_admin(&state, &headers).await?;
    let Some(ref p) = state.portainer_integration else {
        return Err(ApiError::NotFound);
    };
    match integrations::portainer_system_status(p).await {
        Ok(json) => {
            let _ = log_admin_audit(
                &state.pool,
                actor,
                "integrations.portainer.health",
                "integration",
                None,
                serde_json::json!({ "ok": true }),
            )
            .await;
            Ok(Json(serde_json::json!({
                "ok": true,
                "portainer": json,
            })))
        }
        Err(e) => {
            let _ = log_admin_audit(
                &state.pool,
                actor,
                "integrations.portainer.health",
                "integration",
                None,
                serde_json::json!({ "ok": false, "error": e }),
            )
            .await;
            Err(ApiError::BadRequest(e.into()))
        }
    }
}

async fn admin_technitium_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let actor = require_platform_admin(&state, &headers).await?;
    if state.technitium_integration.is_none() {
        return Err(ApiError::NotFound);
    }
    let _ = log_admin_audit(
        &state.pool,
        actor,
        "integrations.technitium.status",
        "integration",
        None,
        serde_json::json!({ "ok": true }),
    )
    .await;
    Ok(Json(serde_json::json!({
        "ok": true,
        "configured": true,
        "note": "DNS record automation is not implemented; API credentials are loaded for future use.",
    })))
}

// --- Audit log ---

#[derive(Serialize)]
struct AuditLogRow {
    id: Uuid,
    actor_user_id: Uuid,
    action: String,
    entity_type: String,
    entity_id: Option<Uuid>,
    metadata: serde_json::Value,
    created_at: DateTime<Utc>,
}

async fn list_audit_log(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<AuditLogRow>>, ApiError> {
    let _actor = require_platform_admin(&state, &headers).await?;
    let limit = clamp_limit(q.limit);
    let offset = q.offset.max(0);

    let search = q.q.as_deref().unwrap_or("").trim();
    let rows: Vec<(Uuid, Uuid, String, String, Option<Uuid>, serde_json::Value, DateTime<Utc>)> =
        if search.is_empty() {
            sqlx::query_as(
                r#"SELECT id, actor_user_id, action, entity_type, entity_id, metadata, created_at
                   FROM admin_audit_log
                   ORDER BY created_at DESC
                   LIMIT $1 OFFSET $2"#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?
        } else {
            let like = format!("%{search}%");
            sqlx::query_as(
                r#"SELECT id, actor_user_id, action, entity_type, entity_id, metadata, created_at
                   FROM admin_audit_log
                   WHERE action ILIKE $1 OR entity_type ILIKE $1
                   ORDER BY created_at DESC
                   LIMIT $2 OFFSET $3"#,
            )
            .bind(&like)
            .bind(limit)
            .bind(offset)
            .fetch_all(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?
        };

    Ok(Json(
        rows.into_iter()
            .map(
                |(
                    id,
                    actor_user_id,
                    action,
                    entity_type,
                    entity_id,
                    metadata,
                    created_at,
                )| AuditLogRow {
                    id,
                    actor_user_id,
                    action,
                    entity_type,
                    entity_id,
                    metadata,
                    created_at,
                },
            )
            .collect(),
    ))
}
