use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{delete, get, patch, post, put};
use axum::{Json, Router};
use chrono::{DateTime, Duration, Utc};
use deploywerk_core::{
    ApiTokenSummary, EnvironmentSummary, InvitationPublic, ProjectSummary, TeamRole, TeamSummary,
    TokenScopes, UserSummary,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{
    generate_api_token_value, hash_api_token_raw, require_principal, role_from_db,
};
use crate::error::ApiError;
use crate::rbac::{
    require_org_member, require_team_access_mutate, require_team_access_read, require_team_owner,
    user_is_platform_admin,
};
use crate::mail;
use crate::slug::slugify;
use crate::audit::try_log_team_audit;
use crate::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .merge(crate::admin::routes())
        .merge(crate::organizations::routes())
        .merge(crate::servers::routes())
        .merge(crate::destinations::routes())
        .merge(crate::applications::routes())
        .merge(crate::notifications::routes())
        .merge(crate::team_platform::routes())
        .merge(crate::team_secrets::routes())
        .merge(crate::cli_invoke::routes())
        .route("/api/v1/health", get(health))
        .route("/api/v1/version", get(version))
        .route("/api/v1/bootstrap", get(bootstrap))
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/me", get(me).patch(patch_me))
        .route("/api/v1/me/current-team", put(put_current_team))
        .route(
            "/api/v1/me/current-organization",
            put(put_current_organization),
        )
        .route("/api/v1/teams", get(list_teams))
        .route("/api/v1/teams/{team_id}/members", get(list_team_members))
        .route(
            "/api/v1/teams/{team_id}/transfer-owner",
            post(transfer_team_owner),
        )
        .route(
            "/api/v1/teams/{team_id}",
            patch(patch_team).delete(delete_team),
        )
        .route(
            "/api/v1/teams/{team_id}/projects",
            get(list_projects).post(create_project),
        )
        .route(
            "/api/v1/teams/{team_id}/projects/{project_id}",
            get(get_project).patch(update_project).delete(delete_project),
        )
        .route(
            "/api/v1/teams/{team_id}/projects/{project_id}/environments",
            get(list_environments).post(create_environment),
        )
        .route(
            "/api/v1/teams/{team_id}/projects/{project_id}/environments/{environment_id}",
            get(get_environment)
                .patch(update_environment)
                .delete(delete_environment),
        )
        .route(
            "/api/v1/tokens",
            get(list_api_tokens).post(create_api_token),
        )
        .route("/api/v1/tokens/{token_id}", delete(revoke_api_token))
        .route(
            "/api/v1/teams/{team_id}/invitations",
            get(list_team_invitations).post(create_invitation),
        )
        .route(
            "/api/v1/teams/{team_id}/invitations/{invitation_id}",
            delete(delete_team_invitation),
        )
        .route(
            "/api/v1/teams/{team_id}/members/{user_id}",
            patch(patch_team_member).delete(delete_team_member),
        )
        .route("/api/v1/invitations/{token}", get(get_invitation_public))
        .route(
            "/api/v1/invitations/{token}/accept",
            post(accept_invitation),
        )
        .route(
            "/api/v1/applications/{application_id}/deploy",
            post(crate::applications::deploy_global),
        )
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true }))
}

async fn version() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "name": "deploywerk-api"
    }))
}

#[derive(Serialize)]
struct BootstrapResponse {
    demo_logins_enabled: bool,
    allow_local_password_auth: bool,
    oidc_enabled: bool,
    #[serde(default)]
    platform_docker_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    apps_base_domain: Option<String>,
    /// OIDC issuer base when SSO is enabled (for operator docs / links).
    #[serde(skip_serializing_if = "Option::is_none")]
    authentik_issuer: Option<String>,
    /// Authentik operator UI when derivable (`/if/admin/`) or set via `AUTHENTIK_ADMIN_URL`.
    #[serde(skip_serializing_if = "Option::is_none")]
    idp_admin_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    demo_accounts: Option<Vec<DemoAccountPublic>>,
    #[serde(default)]
    mail_smtp_configured: bool,
    #[serde(default)]
    public_app_url_configured: bool,
}

#[derive(Serialize)]
struct DemoAccountPublic {
    email: String,
    role: String,
    password: String,
}

async fn bootstrap(State(state): State<Arc<AppState>>) -> Result<Json<BootstrapResponse>, ApiError> {
    let enabled = state.demo_logins_public;
    let accounts = if enabled {
        Some(vec![
            DemoAccountPublic {
                email: "owner@demo.deploywerk.local".into(),
                role: "owner".into(),
                password: "DemoOwner1!".into(),
            },
            DemoAccountPublic {
                email: "admin@demo.deploywerk.local".into(),
                role: "admin".into(),
                password: "DemoAdmin1!".into(),
            },
            DemoAccountPublic {
                email: "member@demo.deploywerk.local".into(),
                role: "member".into(),
                password: "DemoMember1!".into(),
            },
            DemoAccountPublic {
                email: "orgadmin@demo.deploywerk.local".into(),
                role: "org_admin_only".into(),
                password: "DemoOrgAdmin1!".into(),
            },
            DemoAccountPublic {
                email: "teamadmin@demo.deploywerk.local".into(),
                role: "team_admin".into(),
                password: "DemoTeamAdmin1!".into(),
            },
            DemoAccountPublic {
                email: "appadmin@demo.deploywerk.local".into(),
                role: "app_admin".into(),
                password: "DemoAppAdmin1!".into(),
            },
        ])
    } else {
        None
    };

    Ok(Json(BootstrapResponse {
        demo_logins_enabled: enabled,
        allow_local_password_auth: state.allow_local_password_auth,
        oidc_enabled: state.oidc.is_some(),
        platform_docker_enabled: state.deploy_worker.platform_docker_enabled,
        apps_base_domain: state.deploy_worker.apps_base_domain.clone(),
        authentik_issuer: state.oidc.as_ref().map(|o| o.issuer.clone()),
        idp_admin_url: state.idp_admin_url.clone(),
        demo_accounts: accounts,
        mail_smtp_configured: state.smtp_settings.is_some(),
        public_app_url_configured: state.public_app_url.is_some(),
    }))
}

#[derive(Deserialize)]
struct RegisterBody {
    email: String,
    password: String,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserSummary,
}

async fn register(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegisterBody>,
) -> Result<(StatusCode, Json<AuthResponse>), ApiError> {
    use crate::auth::{hash_password, issue_token};

    if !state.allow_local_password_auth {
        return Err(ApiError::Forbidden);
    }

    let email = body.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(ApiError::BadRequest("invalid email".into()));
    }
    if body.password.len() < 8 {
        return Err(ApiError::BadRequest("password too short".into()));
    }

    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM users WHERE email = $1")
        .bind(&email)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    if exists > 0 {
        return Err(ApiError::Conflict("email already registered".into()));
    }

    let id = Uuid::new_v4();
    let hash = hash_password(&body.password)?;
    let now = Utc::now();

    let mut tx = state.pool.begin().await.map_err(|_| ApiError::Internal)?;

    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, created_at) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(&email)
    .bind(&hash)
    .bind(&body.name)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;

    let org_id = Uuid::new_v4();
    let org_slug = format!("org-{}", org_id.simple());
    let org_name = body
        .name
        .as_ref()
        .map(|n| format!("{n}'s organization"))
        .unwrap_or_else(|| format!("{}'s organization", email.split('@').next().unwrap_or("user")));

    sqlx::query("INSERT INTO organizations (id, name, slug, created_at) VALUES ($1, $2, $3, $4)")
        .bind(org_id)
        .bind(&org_name)
        .bind(&org_slug)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?;

    sqlx::query(
        "INSERT INTO organization_memberships (user_id, organization_id, role) VALUES ($1, $2, 'owner')",
    )
    .bind(id)
    .bind(org_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;

    let team_id = Uuid::new_v4();
    let slug = format!("team-{}", team_id.simple());
    let team_name = body
        .name
        .as_ref()
        .map(|n| format!("{n}'s team"))
        .unwrap_or_else(|| format!("{}'s team", email.split('@').next().unwrap_or("user")));

    sqlx::query(
        "INSERT INTO teams (id, organization_id, name, slug, created_at) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(team_id)
    .bind(org_id)
    .bind(&team_name)
    .bind(&slug)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;

    sqlx::query("INSERT INTO team_memberships (user_id, team_id, role) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(team_id)
        .bind(TeamRole::Owner.as_str())
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?;

    sqlx::query(
        r#"INSERT INTO user_preferences (user_id, current_team_id, current_organization_id) VALUES ($1, $2, $3)"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(org_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;

    tx.commit().await.map_err(|_| ApiError::Internal)?;

    let token = issue_token(id, &state.jwt_secret)?;
    let user = crate::rbac::user_summary_with_rbac(
        &state.pool,
        UserSummary {
            id,
            email,
            name: body.name,
            current_team_id: Some(team_id),
            current_organization_id: Some(org_id),
            settings: Some(serde_json::json!({})),
            is_platform_admin: false,
            organization_admin_organization_ids: vec![],
            application_memberships: vec![],
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(AuthResponse { token, user })))
}

#[derive(Deserialize)]
struct LoginBody {
    email: String,
    password: String,
    #[serde(default)]
    totp_code: Option<String>,
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginBody>,
) -> Result<Json<AuthResponse>, ApiError> {
    use crate::auth::{issue_token, verify_password};

    let email = body.email.trim().to_lowercase();
    let row: Option<(Uuid, Option<String>, Option<String>, bool)> =
        sqlx::query_as("SELECT id, password_hash, name, is_platform_admin FROM users WHERE email = $1")
            .bind(&email)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;

    let Some((id, hash, name, is_platform_admin)) = row else {
        return Err(ApiError::Unauthorized);
    };

    let Some(ref hash) = hash else {
        return Err(ApiError::Unauthorized);
    };

    if !verify_password(&body.password, hash) {
        return Err(ApiError::Unauthorized);
    }

    // Org-level MFA enforcement (Phase 1): if any org the user belongs to requires MFA, require a valid TOTP code.
    let mfa_required_any: bool = sqlx::query_scalar(
        r#"SELECT EXISTS(
             SELECT 1
             FROM organization_memberships om
             JOIN organizations o ON o.id = om.organization_id
             WHERE om.user_id = $1 AND o.mfa_required = TRUE
           )"#,
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    if mfa_required_any {
        let totp_row: Option<(Vec<u8>, bool)> = sqlx::query_as(
            "SELECT secret_ciphertext, enabled FROM user_totp WHERE user_id = $1",
        )
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

        let Some((secret_ct, enabled)) = totp_row else {
            return Err(ApiError::Forbidden);
        };
        if !enabled {
            return Err(ApiError::Forbidden);
        }
        let code = body.totp_code.as_deref().unwrap_or("").trim();
        if code.is_empty() {
            return Err(ApiError::Unauthorized);
        }
        let plain = crate::crypto_keys::decrypt_private_key(&state.server_key_encryption_key, &secret_ct)
            .map_err(|_| ApiError::Internal)?;
        let secret_base32 = String::from_utf8(plain).map_err(|_| ApiError::Internal)?;
        let totp = totp_rs::TOTP::new(
            totp_rs::Algorithm::SHA1,
            6,
            1,
            30,
            totp_rs::Secret::Encoded(secret_base32)
                .to_bytes()
                .map_err(|_| ApiError::Internal)?,
            None,
            "DeployWerk".into(),
        )
        .map_err(|_| ApiError::Internal)?;
        let ok = totp.check_current(code).unwrap_or(false);
        if !ok {
            return Err(ApiError::Unauthorized);
        }
    }

    let token = issue_token(id, &state.jwt_secret)?;

    let pref: Option<(Option<Uuid>, Option<Uuid>, serde_json::Value)> = sqlx::query_as(
        "SELECT current_team_id, current_organization_id, COALESCE(settings_json, '{}'::jsonb) FROM user_preferences WHERE user_id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let (current_team_id, current_organization_id, settings) = pref
        .map(|(t, o, s)| (t, o, Some(s)))
        .unwrap_or((None, None, Some(serde_json::json!({}))));

    let user = crate::rbac::user_summary_with_rbac(
        &state.pool,
        UserSummary {
            id,
            email,
            name,
            current_team_id,
            current_organization_id,
            settings,
            is_platform_admin,
            organization_admin_organization_ids: vec![],
            application_memberships: vec![],
        },
    )
    .await?;
    Ok(Json(AuthResponse { token, user }))
}

async fn me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<UserSummary>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    let row: Option<(
        Uuid,
        String,
        Option<String>,
        Option<Uuid>,
        Option<Uuid>,
        serde_json::Value,
        bool,
    )> = sqlx::query_as(
        r#"SELECT u.id, u.email, u.name, up.current_team_id, up.current_organization_id,
                  COALESCE(up.settings_json, '{}'::jsonb), u.is_platform_admin
           FROM users u
           LEFT JOIN user_preferences up ON up.user_id = u.id
           WHERE u.id = $1"#,
    )
    .bind(p.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((id, email, name, current_team_id, current_organization_id, settings_json, is_platform_admin)) =
        row
    else {
        return Err(ApiError::Unauthorized);
    };

    let u = UserSummary {
        id,
        email,
        name,
        current_team_id,
        current_organization_id,
        settings: Some(settings_json),
        is_platform_admin,
        organization_admin_organization_ids: vec![],
        application_memberships: vec![],
    };
    Ok(Json(crate::rbac::user_summary_with_rbac(&state.pool, u).await?))
}

#[derive(Deserialize)]
struct PatchMeBody {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    current_password: Option<String>,
    #[serde(default)]
    new_password: Option<String>,
    #[serde(default)]
    settings: Option<serde_json::Value>,
}

async fn patch_me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<PatchMeBody>,
) -> Result<Json<UserSummary>, ApiError> {
    use crate::auth::{hash_password, verify_password};

    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;

    if body.new_password.is_some() ^ body.current_password.is_some() {
        return Err(ApiError::BadRequest("current_password and new_password must be sent together".into()));
    }

    if let Some(ref np) = body.new_password {
        if np.len() < 8 {
            return Err(ApiError::BadRequest("password too short".into()));
        }
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT password_hash FROM users WHERE id = $1")
                .bind(p.user_id)
                .fetch_optional(&state.pool)
                .await
                .map_err(|_| ApiError::Internal)?;
        let Some((Some(hash),)) = row else {
            return Err(ApiError::BadRequest(
                "password change not available for SSO-only accounts".into(),
            ));
        };
        let cur = body.current_password.as_deref().unwrap_or("");
        if !verify_password(cur, &hash) {
            return Err(ApiError::Unauthorized);
        }
        let new_h = hash_password(np)?;
        sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
            .bind(&new_h)
            .bind(p.user_id)
            .execute(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
    }

    if let Some(ref n) = body.name {
        sqlx::query("UPDATE users SET name = $1 WHERE id = $2")
            .bind(n)
            .bind(p.user_id)
            .execute(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
    }

    if let Some(settings) = body.settings {
        sqlx::query(
            r#"INSERT INTO user_preferences (user_id, settings_json) VALUES ($1, $2)
               ON CONFLICT (user_id) DO UPDATE SET settings_json = EXCLUDED.settings_json"#,
        )
        .bind(p.user_id)
        .bind(settings)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    }

    me(State(state), headers).await
}

#[derive(Deserialize)]
struct CurrentTeamBody {
    team_id: Uuid,
}

async fn put_current_team(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CurrentTeamBody>,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;

    require_team_access_read(&state.pool, p.user_id, body.team_id).await?;

    sqlx::query(
        r#"
        INSERT INTO user_preferences (user_id, current_team_id, current_organization_id) VALUES (
            $1, $2, (SELECT organization_id FROM teams WHERE id = $2)
        )
        ON CONFLICT (user_id) DO UPDATE SET
            current_team_id = EXCLUDED.current_team_id,
            current_organization_id = EXCLUDED.current_organization_id
        "#,
    )
    .bind(p.user_id)
    .bind(body.team_id)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct CurrentOrganizationBody {
    organization_id: Uuid,
}

async fn put_current_organization(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CurrentOrganizationBody>,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;

    require_org_member(&state.pool, p.user_id, body.organization_id).await?;

    let cur_team: Option<Option<Uuid>> =
        sqlx::query_scalar("SELECT current_team_id FROM user_preferences WHERE user_id = $1")
            .bind(p.user_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;

    let keep_team = if let Some(Some(ct)) = cur_team {
        let org_of: Option<Uuid> =
            sqlx::query_scalar("SELECT organization_id FROM teams WHERE id = $1")
                .bind(ct)
                .fetch_optional(&state.pool)
                .await
                .map_err(|_| ApiError::Internal)?;
        if org_of == Some(body.organization_id) {
            Some(ct)
        } else {
            None
        }
    } else {
        None
    };

    let next_team: Option<Uuid> = if let Some(ct) = keep_team {
        Some(ct)
    } else {
        sqlx::query_scalar(
            r#"SELECT t.id FROM teams t
               JOIN team_memberships m ON m.team_id = t.id
               WHERE t.organization_id = $1 AND m.user_id = $2
               ORDER BY t.name LIMIT 1"#,
        )
        .bind(body.organization_id)
        .bind(p.user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?
    };

    sqlx::query(
        r#"
        INSERT INTO user_preferences (user_id, current_organization_id, current_team_id) VALUES ($1, $2, $3)
        ON CONFLICT (user_id) DO UPDATE SET
            current_organization_id = EXCLUDED.current_organization_id,
            current_team_id = EXCLUDED.current_team_id
        "#,
    )
    .bind(p.user_id)
    .bind(body.organization_id)
    .bind(next_team)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn list_teams(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamSummary>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;

    let rows: Vec<(Uuid, Uuid, String, String, String)> = sqlx::query_as(
        r#"
        SELECT t.id, t.organization_id, t.name, t.slug, m.role
        FROM teams t
        JOIN team_memberships m ON m.team_id = t.id AND m.user_id = $1
        ORDER BY t.name
        "#,
    )
    .bind(p.user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let extra: Vec<(Uuid, Uuid, String, String)> = sqlx::query_as(
        r#"SELECT t.id, t.organization_id, t.name, t.slug
           FROM teams t
           JOIN organization_memberships om ON om.organization_id = t.organization_id
           WHERE om.user_id = $1 AND om.role IN ('owner', 'admin')
             AND NOT EXISTS (
               SELECT 1 FROM team_memberships m
               WHERE m.team_id = t.id AND m.user_id = $1
             )
           ORDER BY t.name"#,
    )
    .bind(p.user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let mut out = Vec::with_capacity(rows.len() + extra.len());
    for (tid, oid, name, slug, role) in rows {
        out.push(TeamSummary {
            id: tid,
            organization_id: oid,
            name,
            slug,
            role: role_from_db(&role),
            access_via_organization_admin: false,
        });
    }
    for (tid, oid, name, slug) in extra {
        out.push(TeamSummary {
            id: tid,
            organization_id: oid,
            name,
            slug,
            role: TeamRole::Admin,
            access_via_organization_admin: true,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(Json(out))
}

#[derive(Serialize)]
struct TeamMemberRow {
    user_id: Uuid,
    email: String,
    name: Option<String>,
    role: TeamRole,
}

async fn list_team_members(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamMemberRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_access_read(&state.pool, p.user_id, team_id).await?;

    let rows: Vec<(Uuid, String, Option<String>, String)> = sqlx::query_as(
        r#"SELECT u.id, u.email, u.name, m.role
           FROM team_memberships m
           JOIN users u ON u.id = m.user_id
           WHERE m.team_id = $1
           ORDER BY u.email"#,
    )
    .bind(team_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(|(user_id, email, name, role)| TeamMemberRow {
                user_id,
                email,
                name,
                role: role_from_db(&role),
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct TransferOwnerBody {
    new_owner_user_id: Uuid,
}

async fn transfer_team_owner(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<TransferOwnerBody>,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;
    require_team_owner(&state.pool, p.user_id, team_id).await?;

    if body.new_owner_user_id == p.user_id {
        return Err(ApiError::BadRequest("already owner".into()));
    }

    let in_team: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM team_memberships WHERE team_id = $1 AND user_id = $2",
    )
    .bind(team_id)
    .bind(body.new_owner_user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    if in_team == 0 {
        return Err(ApiError::BadRequest("user is not a member of this team".into()));
    }

    let mut tx = state.pool.begin().await.map_err(|_| ApiError::Internal)?;
    sqlx::query(
        "UPDATE team_memberships SET role = 'admin' WHERE team_id = $1 AND user_id = $2 AND role = 'owner'",
    )
    .bind(team_id)
    .bind(p.user_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;
    let r = sqlx::query(
        "UPDATE team_memberships SET role = 'owner' WHERE team_id = $1 AND user_id = $2",
    )
    .bind(team_id)
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

#[derive(Deserialize)]
struct PatchTeamBody {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    slug: Option<String>,
}

async fn patch_team(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<PatchTeamBody>,
) -> Result<Json<TeamSummary>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;

    if body.name.is_none() && body.slug.is_none() {
        return Err(ApiError::BadRequest("no fields to update".into()));
    }

    let row: Option<(Uuid, String, String)> = sqlx::query_as(
        "SELECT organization_id, name, slug FROM teams WHERE id = $1",
    )
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    let Some((org_id, cur_name, cur_slug)) = row else {
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
            "SELECT COUNT(1) FROM teams WHERE organization_id = $1 AND slug = $2 AND id <> $3",
        )
        .bind(org_id)
        .bind(&t)
        .bind(team_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
        if dup > 0 {
            return Err(ApiError::Conflict(
                "team slug already taken in this organization".into(),
            ));
        }
        t
    } else {
        cur_slug
    };

    sqlx::query("UPDATE teams SET name = $1, slug = $2 WHERE id = $3")
        .bind(&name)
        .bind(&slug)
        .bind(team_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    let in_team: Option<String> = sqlx::query_scalar(
        "SELECT role FROM team_memberships WHERE team_id = $1 AND user_id = $2",
    )
    .bind(team_id)
    .bind(p.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let (role, access_via_organization_admin) = if let Some(ref role_s) = in_team {
        (role_from_db(role_s), false)
    } else if user_is_platform_admin(&state.pool, p.user_id).await? {
        (TeamRole::Admin, false)
    } else {
        (TeamRole::Admin, true)
    };

    Ok(Json(TeamSummary {
        id: team_id,
        organization_id: org_id,
        name,
        slug,
        role,
        access_via_organization_admin,
    }))
}

async fn delete_team(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;
    require_team_owner(&state.pool, p.user_id, team_id).await?;

    let r = sqlx::query("DELETE FROM teams WHERE id = $1")
        .bind(team_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    if r.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn list_projects(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProjectSummary>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_access_read(&state.pool, p.user_id, team_id).await?;

    let rows: Vec<(Uuid, Uuid, String, String, Option<String>, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, team_id, name, slug, description, created_at FROM projects WHERE team_id = $1 ORDER BY name",
    )
    .bind(team_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let mut out = Vec::new();
    for (id, tid, name, slug, desc, created_at) in rows {
        out.push(ProjectSummary {
            id,
            team_id: tid,
            name,
            slug,
            description: desc,
            created_at,
        });
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
struct CreateProjectBody {
    name: String,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

async fn create_project(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<CreateProjectBody>,
) -> Result<(StatusCode, Json<ProjectSummary>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;

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

    let id = Uuid::new_v4();
    let now = Utc::now();

    let r = sqlx::query(
        "INSERT INTO projects (id, team_id, name, slug, description, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(team_id)
    .bind(name)
    .bind(&slug)
    .bind(&body.description)
    .bind(now)
    .execute(&state.pool)
    .await;

    if r.is_err() {
        return Err(ApiError::Conflict("slug already exists".into()));
    }

    Ok((
        StatusCode::CREATED,
        Json(ProjectSummary {
            id,
            team_id,
            name: name.to_string(),
            slug,
            description: body.description,
            created_at: now,
        }),
    ))
}

async fn get_project(
    State(state): State<Arc<AppState>>,
    Path((team_id, project_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<ProjectSummary>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_access_read(&state.pool, p.user_id, team_id).await?;

    let row: Option<(Uuid, Uuid, String, String, Option<String>, DateTime<Utc>)> =
        sqlx::query_as(
            "SELECT id, team_id, name, slug, description, created_at FROM projects WHERE id = $1",
        )
        .bind(project_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    let Some((id, tid, name, slug, desc, created_at)) = row else {
        return Err(ApiError::NotFound);
    };

    if tid != team_id {
        return Err(ApiError::Forbidden);
    }

    Ok(Json(ProjectSummary {
        id,
        team_id: tid,
        name,
        slug,
        description: desc,
        created_at,
    }))
}

#[derive(Deserialize)]
struct UpdateProjectBody {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

async fn update_project(
    State(state): State<Arc<AppState>>,
    Path((team_id, project_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(body): Json<UpdateProjectBody>,
) -> Result<Json<ProjectSummary>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;

    let row: Option<(Uuid, Uuid, String, String, Option<String>, DateTime<Utc>)> =
        sqlx::query_as(
            "SELECT id, team_id, name, slug, description, created_at FROM projects WHERE id = $1",
        )
        .bind(project_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    let Some((id, tid, mut name, mut slug, mut desc, created_at)) = row else {
        return Err(ApiError::NotFound);
    };

    if tid != team_id {
        return Err(ApiError::Forbidden);
    }

    if let Some(n) = body.name.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        name = n.to_string();
    }
    if let Some(s) = body.slug.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        slug = s.to_string();
    }
    if body.description.is_some() {
        desc = body.description.clone();
    }

    sqlx::query(
        "UPDATE projects SET name = $1, slug = $2, description = $3 WHERE id = $4 AND team_id = $5",
    )
    .bind(&name)
    .bind(&slug)
    .bind(&desc)
    .bind(project_id)
    .bind(team_id)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Conflict("slug conflict".into()))?;

    Ok(Json(ProjectSummary {
        id,
        team_id: tid,
        name,
        slug,
        description: desc,
        created_at,
    }))
}

async fn delete_project(
    State(state): State<Arc<AppState>>,
    Path((team_id, project_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;

    let n = sqlx::query("DELETE FROM projects WHERE id = $1 AND team_id = $2")
        .bind(project_id)
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

async fn list_environments(
    State(state): State<Arc<AppState>>,
    Path((team_id, project_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<Vec<EnvironmentSummary>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_access_read(&state.pool, p.user_id, team_id).await?;
    ensure_project_in_team(&state.pool, project_id, team_id).await?;

    let rows: Vec<(
        Uuid,
        Uuid,
        String,
        String,
        DateTime<Utc>,
        bool,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT id, project_id, name, slug, created_at, deploy_locked, deploy_lock_reason, deploy_schedule_json FROM environments WHERE project_id = $1 ORDER BY name",
    )
    .bind(project_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let mut out = Vec::new();
    for (
        id,
        pid,
        name,
        slug,
        created_at,
        deploy_locked,
        deploy_lock_reason,
        deploy_schedule_json,
    ) in rows
    {
        out.push(EnvironmentSummary {
            id,
            project_id: pid,
            name,
            slug,
            created_at,
            deploy_locked,
            deploy_lock_reason,
            deploy_schedule_json,
        });
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
struct CreateEnvironmentBody {
    name: String,
    #[serde(default)]
    slug: Option<String>,
}

async fn create_environment(
    State(state): State<Arc<AppState>>,
    Path((team_id, project_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(body): Json<CreateEnvironmentBody>,
) -> Result<(StatusCode, Json<EnvironmentSummary>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;
    ensure_project_in_team(&state.pool, project_id, team_id).await?;

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

    let id = Uuid::new_v4();
    let now = Utc::now();

    let r = sqlx::query(
        "INSERT INTO environments (id, project_id, name, slug, created_at) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(project_id)
    .bind(name)
    .bind(&slug)
    .bind(now)
    .execute(&state.pool)
    .await;

    if r.is_err() {
        return Err(ApiError::Conflict("slug already exists".into()));
    }

    Ok((
        StatusCode::CREATED,
        Json(EnvironmentSummary {
            id,
            project_id,
            name: name.to_string(),
            slug,
            created_at: now,
            deploy_locked: false,
            deploy_lock_reason: None,
            deploy_schedule_json: None,
        }),
    ))
}

async fn get_environment(
    State(state): State<Arc<AppState>>,
    Path((team_id, project_id, environment_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<EnvironmentSummary>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_access_read(&state.pool, p.user_id, team_id).await?;
    ensure_project_in_team(&state.pool, project_id, team_id).await?;

    let row: Option<(
        Uuid,
        Uuid,
        String,
        String,
        DateTime<Utc>,
        bool,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT id, project_id, name, slug, created_at, deploy_locked, deploy_lock_reason, deploy_schedule_json FROM environments WHERE id = $1",
    )
    .bind(environment_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((
        id,
        pid,
        name,
        slug,
        created_at,
        deploy_locked,
        deploy_lock_reason,
        deploy_schedule_json,
    )) = row
    else {
        return Err(ApiError::NotFound);
    };

    if pid != project_id {
        return Err(ApiError::Forbidden);
    }

    Ok(Json(EnvironmentSummary {
        id,
        project_id: pid,
        name,
        slug,
        created_at,
        deploy_locked,
        deploy_lock_reason,
        deploy_schedule_json,
    }))
}

#[derive(Deserialize)]
struct UpdateEnvironmentBody {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    deploy_locked: Option<bool>,
    #[serde(default)]
    deploy_lock_reason: Option<String>,
    /// JSON string, e.g. `{"utc_start_hour":9,"utc_end_hour":18,"weekdays_only":true}`
    #[serde(default)]
    deploy_schedule_json: Option<String>,
}

async fn update_environment(
    State(state): State<Arc<AppState>>,
    Path((team_id, project_id, environment_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Json(body): Json<UpdateEnvironmentBody>,
) -> Result<Json<EnvironmentSummary>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;
    ensure_project_in_team(&state.pool, project_id, team_id).await?;

    let row: Option<(
        Uuid,
        Uuid,
        String,
        String,
        DateTime<Utc>,
        bool,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT id, project_id, name, slug, created_at, deploy_locked, deploy_lock_reason, deploy_schedule_json FROM environments WHERE id = $1",
    )
    .bind(environment_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((
        id,
        pid,
        mut name,
        mut slug,
        created_at,
        mut deploy_locked,
        mut deploy_lock_reason,
        mut deploy_schedule_json,
    )) = row
    else {
        return Err(ApiError::NotFound);
    };

    if pid != project_id {
        return Err(ApiError::Forbidden);
    }

    if let Some(n) = body.name.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        name = n.to_string();
    }
    if let Some(s) = body.slug.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        slug = s.to_string();
    }
    if let Some(l) = body.deploy_locked {
        deploy_locked = l;
    }
    if let Some(ref r) = body.deploy_lock_reason {
        let t = r.trim();
        deploy_lock_reason = if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        };
    }
    if let Some(ref s) = body.deploy_schedule_json {
        let t = s.trim();
        deploy_schedule_json = if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        };
    }

    sqlx::query(
        "UPDATE environments SET name = $1, slug = $2, deploy_locked = $3, deploy_lock_reason = $4, deploy_schedule_json = $5 WHERE id = $6 AND project_id = $7",
    )
    .bind(&name)
    .bind(&slug)
    .bind(deploy_locked)
    .bind(&deploy_lock_reason)
    .bind(&deploy_schedule_json)
    .bind(environment_id)
    .bind(project_id)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Conflict("slug conflict".into()))?;

    Ok(Json(EnvironmentSummary {
        id,
        project_id: pid,
        name,
        slug,
        created_at,
        deploy_locked,
        deploy_lock_reason,
        deploy_schedule_json,
    }))
}

async fn delete_environment(
    State(state): State<Arc<AppState>>,
    Path((team_id, project_id, environment_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;
    ensure_project_in_team(&state.pool, project_id, team_id).await?;

    let n = sqlx::query("DELETE FROM environments WHERE id = $1 AND project_id = $2")
        .bind(environment_id)
        .bind(project_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?
        .rows_affected();

    if n == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn ensure_project_in_team(
    pool: &sqlx::PgPool,
    project_id: Uuid,
    team_id: Uuid,
) -> Result<(), ApiError> {
    let tid: Option<Uuid> = sqlx::query_scalar("SELECT team_id FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    let Some(t) = tid else {
        return Err(ApiError::NotFound);
    };
    if t != team_id {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

#[derive(Deserialize)]
struct CreateTokenBody {
    name: String,
    #[serde(default)]
    scopes: Vec<String>,
    /// Optional: expire token after N days (1–3650). When omitted, token does not expire.
    #[serde(default)]
    expires_in_days: Option<i64>,
    /// Optional CIDR list (e.g. `["203.0.113.0/24","2001:db8::/32"]`). Empty = no restriction.
    #[serde(default)]
    allowed_cidrs: Option<Vec<String>>,
}

#[derive(Serialize)]
struct CreateTokenResponse {
    token: String,
    id: Uuid,
    name: String,
    scopes: TokenScopes,
    created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_cidrs: Option<Vec<String>>,
}

async fn create_api_token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateTokenBody>,
) -> Result<(StatusCode, Json<CreateTokenResponse>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;

    let name = body.name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest("name required".into()));
    }
    let scopes = TokenScopes::from_list(&body.scopes);
    if !scopes.read && !scopes.write && !scopes.deploy {
        return Err(ApiError::BadRequest("at least one scope required".into()));
    }

    let raw = generate_api_token_value();
    let hash = hash_api_token_raw(&raw);
    let id = Uuid::new_v4();
    let now = Utc::now();
    let scopes_json = scopes.to_json_string();
    let expires_at = body
        .expires_in_days
        .map(|d| d.clamp(1, 3650))
        .map(|d| now + Duration::days(d));

    let allowed_cidrs_out: Option<Vec<String>> = body
        .allowed_cidrs
        .as_ref()
        .map(|list| {
            list.iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|list| !list.is_empty());

    let allowed_json: Option<serde_json::Value> = if let Some(ref list) = allowed_cidrs_out {
        for c in list {
            if c.parse::<ipnetwork::IpNetwork>().is_err() {
                return Err(ApiError::BadRequest(format!("invalid CIDR `{c}`")));
            }
        }
        Some(serde_json::Value::Array(
            list.iter().cloned().map(serde_json::Value::String).collect(),
        ))
    } else {
        None
    };

    sqlx::query(
        "INSERT INTO api_tokens (id, user_id, token_hash, name, scopes, created_at, expires_at, allowed_cidrs) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(id)
    .bind(p.user_id)
    .bind(&hash)
    .bind(name)
    .bind(&scopes_json)
    .bind(now)
    .bind(expires_at)
    .bind(&allowed_json)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok((
        StatusCode::CREATED,
        Json(CreateTokenResponse {
            token: raw,
            id,
            name: name.to_string(),
            scopes,
            created_at: now,
            expires_at,
            allowed_cidrs: allowed_cidrs_out,
        }),
    ))
}

async fn list_api_tokens(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ApiTokenSummary>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_read()?;

    let rows: Vec<(Uuid, String, String, DateTime<Utc>, Option<DateTime<Utc>>, Option<serde_json::Value>)> = sqlx::query_as(
        "SELECT id, name, scopes, created_at, expires_at, allowed_cidrs FROM api_tokens WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(p.user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let mut out = Vec::new();
    for (id, name, scopes_json, created_at, expires_at, allowed_cidrs) in rows {
        let allowed_parsed: Option<Vec<String>> = allowed_cidrs.and_then(|v| {
            v.as_array().map(|a| {
                a.iter()
                    .filter_map(|e| e.as_str().map(|s| s.trim().to_string()))
                    .filter(|s| !s.is_empty())
                    .collect()
            })
        });
        out.push(ApiTokenSummary {
            id,
            name,
            scopes: TokenScopes::parse_json(&scopes_json),
            created_at,
            expires_at,
            allowed_cidrs: allowed_parsed.filter(|v| !v.is_empty()),
        });
    }
    Ok(Json(out))
}

async fn revoke_api_token(
    State(state): State<Arc<AppState>>,
    Path(token_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;

    let n = sqlx::query("DELETE FROM api_tokens WHERE id = $1 AND user_id = $2")
        .bind(token_id)
        .bind(p.user_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?
        .rows_affected();

    if n == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct CreateInvitationBody {
    email: String,
    role: TeamRole,
}

async fn create_invitation(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<CreateInvitationBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;
    if body.role == TeamRole::Owner {
        require_team_owner(&state.pool, p.user_id, team_id).await?;
    }

    let email = body.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(ApiError::BadRequest("invalid email".into()));
    }

    let recent_invites: (i64,) = sqlx::query_as(
        r#"SELECT COUNT(*)::bigint FROM invitations
           WHERE team_id = $1 AND created_at > NOW() - INTERVAL '1 hour'"#,
    )
    .bind(team_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    let skip_invite_email = recent_invites.0 >= 50;

    let token_str = format!("inv_{}", Uuid::new_v4().simple());
    let id = Uuid::new_v4();
    let expires = Utc::now() + Duration::days(14);
    let now = Utc::now();
    let exp_str = expires.to_rfc3339();
    let role_s = body.role.as_str();

    sqlx::query(
        r#"INSERT INTO invitations (id, token, team_id, email, role, expires_at, accepted_at, created_at)
           VALUES ($1, $2, $3, $4, $5, $6, NULL, $7)"#,
    )
    .bind(id)
    .bind(&token_str)
    .bind(team_id)
    .bind(&email)
    .bind(role_s)
    .bind(expires)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let mut invite_email_sent = false;
    if let (Some(ref smtp), Some(ref base)) = (&state.smtp_settings, &state.public_app_url) {
        if skip_invite_email {
            tracing::warn!(
                team_id = %team_id,
                "invite email skipped (team invitation rate limit, last hour)"
            );
        } else {
            let team_name: Option<(String,)> =
                sqlx::query_as("SELECT name FROM teams WHERE id = $1")
                    .bind(team_id)
                    .fetch_optional(&state.pool)
                    .await
                    .map_err(|_| ApiError::Internal)?;
            let team_display = team_name
                .map(|r| r.0)
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| "Team".to_string());
            match mail::send_invitation_email(
                smtp,
                base,
                &token_str,
                &email,
                &team_display,
                role_s,
            )
            .await
            {
                Ok(()) => invite_email_sent = true,
                Err(e) => tracing::warn!(?e, "invite email failed"),
            }
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": id,
            "token": token_str,
            "email": email,
            "role": role_s,
            "expires_at": exp_str,
            "invite_email_sent": invite_email_sent,
        })),
    ))
}

#[derive(Serialize)]
struct TeamInvitationRow {
    id: Uuid,
    email: String,
    role: TeamRole,
    expires_at: DateTime<Utc>,
    accepted: bool,
}

async fn list_team_invitations(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamInvitationRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;

    let rows: Vec<(Uuid, String, String, DateTime<Utc>, Option<DateTime<Utc>>)> = sqlx::query_as(
        r#"SELECT id, email, role, expires_at, accepted_at FROM invitations
           WHERE team_id = $1 ORDER BY created_at DESC"#,
    )
    .bind(team_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(|(id, email, role_s, expires_at, accepted_at)| TeamInvitationRow {
                id,
                email,
                role: role_from_db(&role_s),
                expires_at,
                accepted: accepted_at.is_some(),
            })
            .collect(),
    ))
}

async fn delete_team_invitation(
    State(state): State<Arc<AppState>>,
    Path((team_id, invitation_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;

    let n = sqlx::query(
        "DELETE FROM invitations WHERE id = $1 AND team_id = $2 AND accepted_at IS NULL",
    )
    .bind(invitation_id)
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

#[derive(Deserialize)]
struct PatchTeamMemberBody {
    role: TeamRole,
}

async fn patch_team_member(
    State(state): State<Arc<AppState>>,
    Path((team_id, user_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(body): Json<PatchTeamMemberBody>,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;

    if body.role == TeamRole::Owner {
        return Err(ApiError::BadRequest("use transfer-owner to assign owner".into()));
    }

    let cur: Option<String> =
        sqlx::query_scalar("SELECT role FROM team_memberships WHERE team_id = $1 AND user_id = $2")
            .bind(team_id)
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;

    let Some(cur_s) = cur else {
        return Err(ApiError::NotFound);
    };

    if role_from_db(&cur_s) == TeamRole::Owner {
        return Err(ApiError::BadRequest("cannot change role of owner; transfer ownership first".into()));
    }

    sqlx::query(
        "UPDATE team_memberships SET role = $1 WHERE team_id = $2 AND user_id = $3",
    )
    .bind(body.role.as_str())
    .bind(team_id)
    .bind(user_id)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    try_log_team_audit(
        &state.pool,
        team_id,
        p.user_id,
        "team_member.role_update",
        "team_member",
        Some(user_id),
        serde_json::json!({ "user_id": user_id, "role": body.role.as_str() }),
        None,
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

async fn delete_team_member(
    State(state): State<Arc<AppState>>,
    Path((team_id, user_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;

    let cur: Option<String> =
        sqlx::query_scalar("SELECT role FROM team_memberships WHERE team_id = $1 AND user_id = $2")
            .bind(team_id)
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;

    let Some(cur_s) = cur else {
        return Err(ApiError::NotFound);
    };

    if role_from_db(&cur_s) == TeamRole::Owner {
        return Err(ApiError::BadRequest("cannot remove team owner; transfer ownership first".into()));
    }

    let mut tx = state.pool.begin().await.map_err(|_| ApiError::Internal)?;

    sqlx::query("DELETE FROM team_memberships WHERE team_id = $1 AND user_id = $2")
        .bind(team_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?;

    let org_id: Uuid = sqlx::query_scalar("SELECT organization_id FROM teams WHERE id = $1")
        .bind(team_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?;

    let still_in_org: i64 = sqlx::query_scalar(
        "SELECT COUNT(1)::bigint FROM team_memberships tm
         JOIN teams t ON t.id = tm.team_id
         WHERE tm.user_id = $1 AND t.organization_id = $2",
    )
    .bind(user_id)
    .bind(org_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;

    if still_in_org == 0 {
        sqlx::query(
            "DELETE FROM organization_memberships WHERE user_id = $1 AND organization_id = $2",
        )
        .bind(user_id)
        .bind(org_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?;
    }

    tx.commit().await.map_err(|_| ApiError::Internal)?;

    try_log_team_audit(
        &state.pool,
        team_id,
        p.user_id,
        "team_member.remove",
        "team_member",
        Some(user_id),
        serde_json::json!({ "user_id": user_id }),
        None,
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

async fn get_invitation_public(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Result<Json<InvitationPublic>, ApiError> {
    let row: Option<(String, String, DateTime<Utc>, Option<DateTime<Utc>>, String, String)> =
        sqlx::query_as(
        r#"
        SELECT i.email, i.role, i.expires_at, i.accepted_at, t.name, t.slug
        FROM invitations i
        JOIN teams t ON t.id = i.team_id
        WHERE i.token = $1
        "#,
    )
    .bind(&token)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((email, role_s, expires_at, accepted_at, team_name, team_slug)) = row else {
        return Err(ApiError::NotFound);
    };

    let role = TeamRole::parse(&role_s).ok_or(ApiError::Internal)?;
    let expired = expires_at < Utc::now();

    Ok(Json(InvitationPublic {
        team_name,
        team_slug,
        email,
        role,
        expires_at,
        accepted: accepted_at.is_some(),
        expired,
    }))
}

async fn accept_invitation(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_jwt()?;
    p.require_read()?;

    let row: Option<(Uuid, Uuid, String, String, DateTime<Utc>, Option<DateTime<Utc>>)> =
        sqlx::query_as(
            "SELECT id, team_id, email, role, expires_at, accepted_at FROM invitations WHERE token = $1",
        )
    .bind(&token)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((inv_id, team_id, invite_email, role_s, exp_at, accepted_at)) = row else {
        return Err(ApiError::NotFound);
    };

    if accepted_at.is_some() {
        return Err(ApiError::Conflict("already accepted".into()));
    }

    if exp_at < Utc::now() {
        return Err(ApiError::BadRequest("expired".into()));
    }

    let user_email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(p.user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    if user_email.to_lowercase() != invite_email.to_lowercase() {
        return Err(ApiError::Forbidden);
    }

    let role = role_from_db(&role_s);

    let mut tx = state.pool.begin().await.map_err(|_| ApiError::Internal)?;

    sqlx::query("UPDATE invitations SET accepted_at = $1 WHERE id = $2")
        .bind(Utc::now())
        .bind(inv_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?;

    sqlx::query(
        r#"INSERT INTO team_memberships (user_id, team_id, role) VALUES ($1, $2, $3)
           ON CONFLICT (user_id, team_id) DO UPDATE SET role = EXCLUDED.role"#,
    )
    .bind(p.user_id)
    .bind(team_id)
    .bind(role.as_str())
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;

    let org_id: Uuid = sqlx::query_scalar("SELECT organization_id FROM teams WHERE id = $1")
        .bind(team_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?;

    let existing_org_role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM organization_memberships WHERE user_id = $1 AND organization_id = $2",
    )
    .bind(p.user_id)
    .bind(org_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;

    let merged_org_role = if let Some(ref rs) = existing_org_role {
        let er = role_from_db(rs);
        TeamRole::max_rank(er, role).as_str()
    } else {
        role.as_str()
    };

    sqlx::query(
        r#"INSERT INTO organization_memberships (user_id, organization_id, role) VALUES ($1, $2, $3)
           ON CONFLICT (user_id, organization_id) DO UPDATE SET role = EXCLUDED.role"#,
    )
    .bind(p.user_id)
    .bind(org_id)
    .bind(merged_org_role)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;

    tx.commit().await.map_err(|_| ApiError::Internal)?;

    Ok(StatusCode::NO_CONTENT)
}

