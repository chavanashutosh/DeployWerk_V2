mod auth;
mod config;
mod error;
mod seed;

use std::sync::Arc;

use axum::extract::State;
use axum::http::{header::AUTHORIZATION, HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use deploywerk_core::{TeamRole, TeamSummary, UserSummary};
use error::ApiError;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::auth::{
    hash_password, issue_token, parse_bearer_token, role_from_db, user_id_from_token,
    verify_password,
};
use crate::config::Config;

#[derive(Clone)]
struct AppState {
    pool: SqlitePool,
    jwt_secret: String,
    demo_logins_public: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "deploywerk_api=info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env();

    let opts = config
        .database_url
        .parse::<SqliteConnectOptions>()?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePool::connect_with(opts).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    if config.seed_demo_users {
        tracing::info!("seeding demo users");
        seed::seed_demo_users(&pool).await?;
    }

    let state = Arc::new(AppState {
        pool,
        jwt_secret: config.jwt_secret.clone(),
        demo_logins_public: config.demo_logins_public,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/version", get(version))
        .route("/api/v1/bootstrap", get(bootstrap))
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/me", get(me))
        .route("/api/v1/teams", get(list_teams))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
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
    #[serde(skip_serializing_if = "Option::is_none")]
    demo_accounts: Option<Vec<DemoAccountPublic>>,
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
        ])
    } else {
        None
    };

    Ok(Json(BootstrapResponse {
        demo_logins_enabled: enabled,
        demo_accounts: accounts,
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
struct AuthResponse {
    token: String,
    user: UserSummary,
}

async fn register(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegisterBody>,
) -> Result<(StatusCode, Json<AuthResponse>), ApiError> {
    let email = body.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(ApiError::BadRequest("invalid email"));
    }
    if body.password.len() < 8 {
        return Err(ApiError::BadRequest("password too short"));
    }

    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM users WHERE email = ?")
        .bind(&email)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;

    if exists > 0 {
        return Err(ApiError::Conflict("email already registered".into()));
    }

    let id = Uuid::new_v4();
    let hash = hash_password(&body.password)?;
    let now = Utc::now().to_rfc3339();

    let mut tx = state.pool.begin().await.map_err(|_| ApiError::Internal)?;

    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(&email)
    .bind(&hash)
    .bind(&body.name)
    .bind(&now)
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

    sqlx::query("INSERT INTO teams (id, name, slug, created_at) VALUES (?, ?, ?, ?)")
        .bind(team_id.to_string())
        .bind(&team_name)
        .bind(&slug)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?;

    sqlx::query("INSERT INTO team_memberships (user_id, team_id, role) VALUES (?, ?, ?)")
        .bind(id.to_string())
        .bind(team_id.to_string())
        .bind(TeamRole::Owner.as_str())
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?;

    tx.commit().await.map_err(|_| ApiError::Internal)?;

    let token = issue_token(id, &state.jwt_secret)?;
    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token,
            user: UserSummary {
                id,
                email,
                name: body.name,
            },
        }),
    ))
}

#[derive(Deserialize)]
struct LoginBody {
    email: String,
    password: String,
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginBody>,
) -> Result<Json<AuthResponse>, ApiError> {
    let email = body.email.trim().to_lowercase();
    let row: Option<(String, String, Option<String>)> =
        sqlx::query_as("SELECT id, password_hash, name FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;

    let Some((id_str, hash, name)) = row else {
        return Err(ApiError::Unauthorized);
    };

    if !verify_password(&body.password, &hash) {
        return Err(ApiError::Unauthorized);
    }

    let id = Uuid::parse_str(&id_str).map_err(|_| ApiError::Internal)?;
    let token = issue_token(id, &state.jwt_secret)?;

    Ok(Json(AuthResponse {
        token,
        user: UserSummary {
            id,
            email,
            name,
        },
    }))
}

async fn me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<UserSummary>, ApiError> {
    let uid = require_user(&state, &headers).await?;
    let row: Option<(String, String, Option<String>)> =
        sqlx::query_as("SELECT id, email, name FROM users WHERE id = ?")
            .bind(uid.to_string())
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;

    let Some((id_str, email, name)) = row else {
        return Err(ApiError::Unauthorized);
    };

    let id = Uuid::parse_str(&id_str).map_err(|_| ApiError::Internal)?;
    Ok(Json(UserSummary { id, email, name }))
}

async fn list_teams(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamSummary>>, ApiError> {
    let uid = require_user(&state, &headers).await?;

    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        r#"
        SELECT t.id, t.name, t.slug, m.role
        FROM teams t
        JOIN team_memberships m ON m.team_id = t.id
        WHERE m.user_id = ?
        ORDER BY t.name
        "#,
    )
    .bind(uid.to_string())
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let mut out = Vec::with_capacity(rows.len());
    for (id, name, slug, role) in rows {
        let tid = Uuid::parse_str(&id).map_err(|_| ApiError::Internal)?;
        out.push(TeamSummary {
            id: tid,
            name,
            slug,
            role: role_from_db(&role),
        });
    }

    Ok(Json(out))
}

async fn require_user(state: &AppState, headers: &HeaderMap) -> Result<Uuid, ApiError> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    let token = parse_bearer_token(auth).ok_or(ApiError::Unauthorized)?;
    user_id_from_token(&state.pool, token, &state.jwt_secret).await
}
