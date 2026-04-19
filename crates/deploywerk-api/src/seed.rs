use chrono::Utc;
use deploywerk_core::{AppRole, TeamRole};
use crate::DbPool;
use uuid::Uuid;

use crate::auth::hash_password;
use crate::error::ApiError;
use crate::crypto_keys::encrypt_private_key;

struct DemoAccount {
    email: &'static str,
    password: &'static str,
    name: &'static str,
    role: TeamRole,
}

const DEMO_ORG_NAME: &str = "Demo Team";
const DEMO_ORG_SLUG: &str = "demo";
const DEMO_TEAM_NAME: &str = "Demo Team";
const DEMO_TEAM_SLUG: &str = "demo";
const DEMO_PROJECT_SLUG: &str = "sample";
const DEMO_ENV_SLUG: &str = "production";
const DEMO_APP_SLUG: &str = "hello";
const DEMO_APP2_SLUG: &str = "api";

pub async fn seed_demo_users(pool: &DbPool, server_key_encryption_key: &[u8; 32]) -> Result<(), ApiError> {
    let demos = [
        DemoAccount {
            email: "owner@demo.deploywerk.local",
            password: "DemoOwner1!",
            name: "Demo Owner",
            role: TeamRole::Owner,
        },
        DemoAccount {
            email: "admin@demo.deploywerk.local",
            password: "DemoAdmin1!",
            name: "Demo Admin",
            role: TeamRole::Admin,
        },
        DemoAccount {
            email: "member@demo.deploywerk.local",
            password: "DemoMember1!",
            name: "Demo Member",
            role: TeamRole::Member,
        },
    ];

    let org_id = if sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM organizations WHERE slug = $1")
        .bind(DEMO_ORG_SLUG)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::Internal)?
        > 0
    {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM organizations WHERE slug = $1 LIMIT 1")
            .bind(DEMO_ORG_SLUG)
            .fetch_one(pool)
            .await
            .map_err(|_| ApiError::Internal)?
    } else {
        let oid = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO organizations (id, name, slug, created_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(oid)
        .bind(DEMO_ORG_NAME)
        .bind(DEMO_ORG_SLUG)
        .bind(now)
        .execute(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
        oid
    };

    let team_id =
        if sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM teams WHERE organization_id = $1 AND slug = $2",
        )
        .bind(org_id)
        .bind(DEMO_TEAM_SLUG)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::Internal)?
            > 0
        {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM teams WHERE organization_id = $1 AND slug = $2 LIMIT 1",
            )
            .bind(org_id)
            .bind(DEMO_TEAM_SLUG)
            .fetch_one(pool)
            .await
            .map_err(|_| ApiError::Internal)?
        } else {
            let tid = Uuid::new_v4();
            let now = Utc::now();
            sqlx::query(
                "INSERT INTO teams (id, organization_id, name, slug, created_at) VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(tid)
            .bind(org_id)
            .bind(DEMO_TEAM_NAME)
            .bind(DEMO_TEAM_SLUG)
            .bind(now)
            .execute(pool)
            .await
            .map_err(|_| ApiError::Internal)?;
            tid
        };

    for d in demos {
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM users WHERE email = $1")
                .bind(d.email)
                .fetch_one(pool)
                .await
                .map_err(|_| ApiError::Internal)?;

        let uid = if count > 0 {
            sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1 LIMIT 1")
                .bind(d.email)
                .fetch_one(pool)
                .await
                .map_err(|_| ApiError::Internal)?
        } else {
            let id = Uuid::new_v4();
            let hash = hash_password(d.password)?;
            let now = Utc::now();
            sqlx::query(
                "INSERT INTO users (id, email, password_hash, name, created_at) VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(id)
            .bind(d.email)
            .bind(&hash)
            .bind(d.name)
            .bind(now)
            .execute(pool)
            .await
            .map_err(|_| ApiError::Internal)?;
            id
        };

        let om = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM organization_memberships WHERE user_id = $1 AND organization_id = $2",
        )
        .bind(uid)
        .bind(org_id)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::Internal)?;

        if om == 0 {
            sqlx::query(
                "INSERT INTO organization_memberships (user_id, organization_id, role) VALUES ($1, $2, $3)",
            )
            .bind(uid)
            .bind(org_id)
            .bind(d.role.as_str())
            .execute(pool)
            .await
            .map_err(|_| ApiError::Internal)?;
        }

        let mem = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM team_memberships WHERE user_id = $1 AND team_id = $2",
        )
        .bind(uid)
        .bind(team_id)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::Internal)?;

        if mem == 0 {
            sqlx::query(
                "INSERT INTO team_memberships (user_id, team_id, role) VALUES ($1, $2, $3)",
            )
            .bind(uid)
            .bind(team_id)
            .bind(d.role.as_str())
            .execute(pool)
            .await
            .map_err(|_| ApiError::Internal)?;
        }
    }

    let hello_app_id = seed_demo_project_env_app(pool, team_id).await?;
    seed_demo_storage_backend(pool, team_id, server_key_encryption_key)
        .await
        .ok();

    seed_rbac_demo_accounts(pool, org_id, team_id, hello_app_id).await?;

    Ok(())
}

async fn upsert_password_user(
    pool: &DbPool,
    email: &str,
    password: &str,
    name: &str,
) -> Result<Uuid, ApiError> {
    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    if count > 0 {
        return sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1 LIMIT 1")
            .bind(email)
            .fetch_one(pool)
            .await
            .map_err(|_| ApiError::Internal);
    }
    let id = Uuid::new_v4();
    let hash = hash_password(password)?;
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, created_at) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(email)
    .bind(&hash)
    .bind(name)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    sqlx::query(
        crate::sql_compat::insert_user_prefs_empty_settings(),
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    Ok(id)
}

/// Extra users for RBAC demos: org-only admin, team admin, app-scoped admin (not platform admin).
async fn seed_rbac_demo_accounts(
    pool: &DbPool,
    org_id: Uuid,
    team_id: Uuid,
    hello_app_id: Uuid,
) -> Result<(), ApiError> {
    // Org owner/admin only — no team membership (governance via organization).
    let org_admin_id = upsert_password_user(
        pool,
        "orgadmin@demo.deploywerk.local",
        "DemoOrgAdmin1!",
        "Demo Org Admin (org only)",
    )
    .await?;
    if sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1) FROM organization_memberships WHERE user_id = $1 AND organization_id = $2",
    )
    .bind(org_admin_id)
    .bind(org_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::Internal)?
        == 0
    {
        sqlx::query(
            "INSERT INTO organization_memberships (user_id, organization_id, role) VALUES ($1, $2, 'admin')",
        )
        .bind(org_admin_id)
        .bind(org_id)
        .execute(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    }

    // Team admin (member of org + team; distinct login for team-focused admin).
    let team_admin_id = upsert_password_user(
        pool,
        "teamadmin@demo.deploywerk.local",
        "DemoTeamAdmin1!",
        "Demo Team Admin",
    )
    .await?;
    if sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1) FROM organization_memberships WHERE user_id = $1 AND organization_id = $2",
    )
    .bind(team_admin_id)
    .bind(org_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::Internal)?
        == 0
    {
        sqlx::query(
            "INSERT INTO organization_memberships (user_id, organization_id, role) VALUES ($1, $2, 'member')",
        )
        .bind(team_admin_id)
        .bind(org_id)
        .execute(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    }
    if sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1) FROM team_memberships WHERE user_id = $1 AND team_id = $2",
    )
    .bind(team_admin_id)
    .bind(team_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::Internal)?
        == 0
    {
        sqlx::query(
            "INSERT INTO team_memberships (user_id, team_id, role) VALUES ($1, $2, 'admin')",
        )
        .bind(team_admin_id)
        .bind(team_id)
        .execute(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    }

    // App-scoped admin: team member, explicit application_memberships.admin on hello app only.
    let app_admin_id = upsert_password_user(
        pool,
        "appadmin@demo.deploywerk.local",
        "DemoAppAdmin1!",
        "Demo App Admin",
    )
    .await?;
    if sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1) FROM organization_memberships WHERE user_id = $1 AND organization_id = $2",
    )
    .bind(app_admin_id)
    .bind(org_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::Internal)?
        == 0
    {
        sqlx::query(
            "INSERT INTO organization_memberships (user_id, organization_id, role) VALUES ($1, $2, 'member')",
        )
        .bind(app_admin_id)
        .bind(org_id)
        .execute(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    }
    if sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1) FROM team_memberships WHERE user_id = $1 AND team_id = $2",
    )
    .bind(app_admin_id)
    .bind(team_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::Internal)?
        == 0
    {
        sqlx::query(
            "INSERT INTO team_memberships (user_id, team_id, role) VALUES ($1, $2, 'member')",
        )
        .bind(app_admin_id)
        .bind(team_id)
        .execute(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    }
    if sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1) FROM application_memberships WHERE user_id = $1 AND application_id = $2",
    )
    .bind(app_admin_id)
    .bind(hello_app_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::Internal)?
        == 0
    {
        sqlx::query(
            "INSERT INTO application_memberships (user_id, application_id, role) VALUES ($1, $2, $3)",
        )
        .bind(app_admin_id)
        .bind(hello_app_id)
        .bind(AppRole::Admin.as_str())
        .execute(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    }

    Ok(())
}

async fn seed_demo_storage_backend(
    pool: &DbPool,
    team_id: Uuid,
    server_key_encryption_key: &[u8; 32],
) -> Result<(), ApiError> {
    let endpoint_url = std::env::var("DEPLOYWERK_DEFAULT_STORAGE_ENDPOINT_URL").unwrap_or_default();
    let bucket = std::env::var("DEPLOYWERK_DEFAULT_STORAGE_BUCKET").unwrap_or_default();
    let access_key = std::env::var("DEPLOYWERK_DEFAULT_STORAGE_ACCESS_KEY").unwrap_or_default();
    let secret_key = std::env::var("DEPLOYWERK_DEFAULT_STORAGE_SECRET_KEY").unwrap_or_default();
    let region = std::env::var("DEPLOYWERK_DEFAULT_STORAGE_REGION").unwrap_or_else(|_| "us-east-1".into());
    let path_style = std::env::var("DEPLOYWERK_DEFAULT_STORAGE_PATH_STYLE")
        .ok()
        .map(|s| s == "1" || s.eq_ignore_ascii_case("true") || s.eq_ignore_ascii_case("yes"))
        .unwrap_or(true);

    if endpoint_url.trim().is_empty()
        || bucket.trim().is_empty()
        || access_key.trim().is_empty()
        || secret_key.trim().is_empty()
    {
        return Ok(());
    }

    let exists: i64 = sqlx::query_scalar("SELECT COUNT(1) FROM storage_backends WHERE team_id = $1")
        .bind(team_id)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    if exists > 0 {
        return Ok(());
    }

    let ak = encrypt_private_key(server_key_encryption_key, access_key.as_bytes())
        .map_err(|_| ApiError::Internal)?;
    let sk = encrypt_private_key(server_key_encryption_key, secret_key.as_bytes())
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
    .bind("Local MinIO")
    .bind(endpoint_url.trim())
    .bind(bucket.trim())
    .bind(region.trim())
    .bind(path_style)
    .bind(&ak)
    .bind(&sk)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(())
}

/// Idempotent sample project / environment / applications for the demo team (UI and API exploration).
/// Returns the `hello` application id (for app-scoped RBAC seeding).
async fn seed_demo_project_env_app(pool: &DbPool, team_id: Uuid) -> Result<Uuid, ApiError> {
    let project_id =
        if sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM projects WHERE team_id = $1 AND slug = $2")
            .bind(team_id)
            .bind(DEMO_PROJECT_SLUG)
            .fetch_one(pool)
            .await
            .map_err(|_| ApiError::Internal)?
            > 0
        {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM projects WHERE team_id = $1 AND slug = $2 LIMIT 1",
            )
            .bind(team_id)
            .bind(DEMO_PROJECT_SLUG)
            .fetch_one(pool)
            .await
            .map_err(|_| ApiError::Internal)?
        } else {
            let pid = Uuid::new_v4();
            let now = Utc::now();
            sqlx::query(
                r#"INSERT INTO projects (id, team_id, name, slug, description, created_at)
                   VALUES ($1, $2, $3, $4, $5, $6)"#,
            )
            .bind(pid)
            .bind(team_id)
            .bind("Sample project")
            .bind(DEMO_PROJECT_SLUG)
            .bind("Seeded demo project for DeployWerk UI exploration")
            .bind(now)
            .execute(pool)
            .await
            .map_err(|_| ApiError::Internal)?;
            pid
        };

    let environment_id = if sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1) FROM environments WHERE project_id = $1 AND slug = $2",
    )
    .bind(project_id)
    .bind(DEMO_ENV_SLUG)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::Internal)?
        > 0
    {
        sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM environments WHERE project_id = $1 AND slug = $2 LIMIT 1",
        )
        .bind(project_id)
        .bind(DEMO_ENV_SLUG)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::Internal)?
    } else {
        let eid = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            r#"INSERT INTO environments (id, project_id, name, slug, created_at)
               VALUES ($1, $2, $3, $4, $5)"#,
        )
        .bind(eid)
        .bind(project_id)
        .bind("Production")
        .bind(DEMO_ENV_SLUG)
        .bind(now)
        .execute(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
        eid
    };

    let app_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(1) FROM applications WHERE environment_id = $1 AND slug = $2",
    )
    .bind(environment_id)
    .bind(DEMO_APP_SLUG)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    if app_exists == 0 {
        let aid = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            r#"INSERT INTO applications
               (id, environment_id, destination_id, name, slug, docker_image, created_at)
               VALUES ($1, $2, NULL, $3, $4, $5, $6)"#,
        )
        .bind(aid)
        .bind(environment_id)
        .bind("Hello (nginx)")
        .bind(DEMO_APP_SLUG)
        .bind("nginx:alpine")
        .bind(now)
        .execute(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    }

    let hello_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM applications WHERE environment_id = $1 AND slug = $2 LIMIT 1",
    )
    .bind(environment_id)
    .bind(DEMO_APP_SLUG)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let app2: i64 = sqlx::query_scalar(
        "SELECT COUNT(1) FROM applications WHERE environment_id = $1 AND slug = $2",
    )
    .bind(environment_id)
    .bind(DEMO_APP2_SLUG)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    if app2 == 0 {
        let aid = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            r#"INSERT INTO applications
               (id, environment_id, destination_id, name, slug, docker_image, created_at)
               VALUES ($1, $2, NULL, $3, $4, $5, $6)"#,
        )
        .bind(aid)
        .bind(environment_id)
        .bind("API (httpd)")
        .bind(DEMO_APP2_SLUG)
        .bind("httpd:alpine")
        .bind(now)
        .execute(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    }

    Ok(hello_id)
}
