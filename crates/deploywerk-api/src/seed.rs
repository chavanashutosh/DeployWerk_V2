use chrono::Utc;
use deploywerk_core::TeamRole;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::auth::hash_password;
use crate::error::ApiError;

struct DemoAccount {
    email: &'static str,
    password: &'static str,
    name: &'static str,
    role: TeamRole,
}

const DEMO_TEAM_NAME: &str = "Demo Team";
const DEMO_TEAM_SLUG: &str = "demo";

pub async fn seed_demo_users(pool: &SqlitePool) -> Result<(), ApiError> {
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

    let team_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM teams WHERE slug = ?")
        .bind(DEMO_TEAM_SLUG)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    let team_id = if team_exists > 0 {
        sqlx::query_scalar::<_, String>("SELECT id FROM teams WHERE slug = ? LIMIT 1")
            .bind(DEMO_TEAM_SLUG)
            .fetch_one(pool)
            .await
            .map_err(|_| ApiError::Internal)?
    } else {
        let tid = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO teams (id, name, slug, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(&tid)
        .bind(DEMO_TEAM_NAME)
        .bind(DEMO_TEAM_SLUG)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
        tid
    };

    for d in demos {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM users WHERE email = ?")
            .bind(d.email)
            .fetch_one(pool)
            .await
            .map_err(|_| ApiError::Internal)?;

        let uid = if count > 0 {
            sqlx::query_scalar::<_, String>("SELECT id FROM users WHERE email = ? LIMIT 1")
                .bind(d.email)
                .fetch_one(pool)
                .await
                .map_err(|_| ApiError::Internal)?
        } else {
            let id = Uuid::new_v4().to_string();
            let hash = hash_password(d.password)?;
            let now = Utc::now().to_rfc3339();
            sqlx::query(
                "INSERT INTO users (id, email, password_hash, name, created_at) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(d.email)
            .bind(&hash)
            .bind(d.name)
            .bind(&now)
            .execute(pool)
            .await
            .map_err(|_| ApiError::Internal)?;
            id
        };

        let mem = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM team_memberships WHERE user_id = ? AND team_id = ?",
        )
        .bind(&uid)
        .bind(&team_id)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::Internal)?;

        if mem == 0 {
            sqlx::query(
                "INSERT INTO team_memberships (user_id, team_id, role) VALUES (?, ?, ?)",
            )
            .bind(&uid)
            .bind(&team_id)
            .bind(d.role.as_str())
            .execute(pool)
            .await
            .map_err(|_| ApiError::Internal)?;
        }
    }

    Ok(())
}
