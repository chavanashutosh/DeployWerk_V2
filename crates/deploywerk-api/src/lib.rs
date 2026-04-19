//! DeployWerk HTTP API library surface (shared with the `deploywerk-deploy-worker` binary).

mod admin;
mod cli_invoke;
mod applications;
mod auth;
mod audit;
mod config;
mod crypto_keys;
mod deploy_schedule;
mod github_app_api;
mod destinations;
mod entitlements;
mod error;
mod mail;
mod mfa;
mod mail_platform;
mod notifications;
mod oidc;
mod permissions_catalog;
mod organizations;
mod rbac;
mod scim;
mod seed;
mod servers;
mod saml;
mod slug;
mod team_platform;
mod team_secrets;
mod webhook_github;
mod handlers;
mod integrations;

pub use applications::{execute_deploy_job, try_claim_next_queued_deploy_job};
pub use config::IntegrationUrls;

use std::sync::Arc;
use std::time::Duration;

use axum::Router;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_http::LatencyUnit;
use tracing::Level;

pub use config::{Config, DeployWorkerConfig};

/// Shared application state for Axum handlers.
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub server_key_encryption_key: [u8; 32],
    pub demo_logins_public: bool,
    pub allow_local_password_auth: bool,
    pub stripe_webhook_secret: Option<String>,
    pub adyen_hmac_key_hex: Option<String>,
    pub cdn_purge_webhook_url: Option<String>,
    pub deploywerk_git_sha: Option<String>,
    pub oidc: Option<oidc::OidcConfigState>,
    pub scim_bearer_token: Option<String>,
    pub scim_idp_issuer: Option<String>,
    pub mollie_api_key: Option<String>,
    pub deploy_worker: DeployWorkerConfig,
    /// When false, deploy jobs stay `queued` until [`applications::execute_deploy_job`] is run by `deploywerk-deploy-worker`.
    pub deploy_dispatch_inline: bool,
    pub github_app_webhook_secret: Option<String>,
    /// Public app slug for `https://github.com/apps/{slug}/installations/new` (operator UX).
    pub github_app_slug: Option<String>,
    /// Authentik admin UI (or other IdP) for operator links from bootstrap / login.
    pub idp_admin_url: Option<String>,
    /// Instance SMTP (transactional mail). None when not configured.
    pub smtp_settings: Option<mail::SmtpSettings>,
    /// Public web origin for invite links in email, e.g. `https://app.example.com`.
    pub public_app_url: Option<String>,
    pub admin_action_emails_enabled: bool,
    /// True when loopback (127.0.0.1) preset URLs were merged (env flag or development default when unset).
    pub local_service_defaults: bool,
    /// Operator integration links (bootstrap); no secrets.
    pub integration_urls: IntegrationUrls,
    /// Optional docs base for SSO section link (`{base}/README.md#single-sign-on-oidc`).
    pub documentation_base_url: Option<String>,
    /// Technitium DNS API (optional automation).
    pub technitium_integration: Option<integrations::TechnitiumIntegration>,
    /// Portainer API read-only probe.
    pub portainer_integration: Option<integrations::PortainerIntegration>,
}

/// Run migrations, optional seed, then the HTTP server (used by `deploywerk-api` binary).
pub async fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "deploywerk_api=info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env();

    let max_connections = std::env::var("DATABASE_MAX_CONNECTIONS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let acquire_timeout_secs = std::env::var("DATABASE_ACQUIRE_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    let statement_timeout_ms: Option<i64> = std::env::var("DATABASE_STATEMENT_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|v| *v > 0);

    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(acquire_timeout_secs))
        .after_connect(move |conn, _meta| {
            let statement_timeout_ms = statement_timeout_ms;
            Box::pin(async move {
                if let Some(ms) = statement_timeout_ms {
                    // Numeric-only to avoid SQL injection risk.
                    let q = format!("SET statement_timeout = {ms}");
                    sqlx::query(&q).execute(conn).await?;
                }
                Ok(())
            })
        })
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    if let Some(ref email) = config.bootstrap_platform_admin_email {
        let e = email.trim();
        if !e.is_empty() {
            let r = sqlx::query(
                "UPDATE users SET is_platform_admin = TRUE WHERE LOWER(email) = LOWER($1)",
            )
            .bind(e)
            .execute(&pool)
            .await?;
            if r.rows_affected() > 0 {
                tracing::info!(email = e, "bootstrap platform admin granted");
            } else {
                tracing::warn!(email = e, "bootstrap platform admin: no user with that email yet");
            }
        }
    }

    if config.seed_demo_users {
        tracing::info!("seeding demo users");
        seed::seed_demo_users(&pool, &config.server_key_encryption_key).await?;
    }

    let oidc = match (
        config.authentik_issuer.clone(),
        config.authentik_client_id.clone(),
        config.authentik_client_secret.clone(),
    ) {
        (Some(iss), Some(cid), Some(sec))
            if !iss.is_empty() && !cid.is_empty() && !sec.is_empty() =>
        {
            let http = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()?;
            Some(oidc::OidcConfigState {
                issuer: iss.clone(),
                idp_issuer_db: iss,
                client_id: cid,
                client_secret: sec,
                redirect_uri: config.authentik_redirect_uri.clone(),
                http,
                metadata: tokio::sync::Mutex::new(None),
            })
        }
        _ => None,
    };

    let scim_bearer_token = config.deploywerk_scim_bearer_token.clone();
    let scim_idp_issuer = if scim_bearer_token.is_some() {
        config.deploywerk_scim_idp_issuer.clone()
    } else {
        None
    };

    let deploy_dispatch_inline = config.deploy_dispatch_inline;

    let technitium_integration = if config.technitium_dns_enabled {
        match (
            config.technitium_api_url.clone(),
            config.technitium_api_token.clone(),
        ) {
            (Some(api_url), Some(api_token)) => Some(integrations::TechnitiumIntegration { api_url, api_token }),
            _ => {
                tracing::warn!(
                    "DEPLOYWERK_TECHNITIUM_DNS_ENABLED but DEPLOYWERK_TECHNITIUM_API_URL or DEPLOYWERK_TECHNITIUM_API_TOKEN missing"
                );
                None
            }
        }
    } else {
        None
    };

    let portainer_api_url = config
        .portainer_api_url
        .clone()
        .or_else(|| config.integration_urls.portainer_url.clone());

    let portainer_integration = if config.portainer_integration_enabled {
        match (portainer_api_url, config.portainer_api_token.clone()) {
            (Some(api_url), Some(api_token)) => Some(integrations::PortainerIntegration { api_url, api_token }),
            _ => {
                tracing::warn!(
                    "DEPLOYWERK_PORTAINER_INTEGRATION_ENABLED but DEPLOYWERK_PORTAINER_API_URL (or INTEGRATION_PORTAINER_URL) and DEPLOYWERK_PORTAINER_API_TOKEN required"
                );
                None
            }
        }
    } else {
        None
    };

    if let (Some(app_id), Some(ref pem)) = (config.github_app_id, config.github_app_private_key_pem.as_ref())
    {
        match github_app_api::encode_github_app_jwt(app_id, pem) {
            Ok(_) => tracing::info!(
                app_id,
                "GitHub App JWT encoding OK (use for installation access token API; webhooks still use HMAC)"
            ),
            Err(e) => tracing::warn!(
                ?e,
                "GITHUB_APP_ID / GITHUB_APP_PRIVATE_KEY_PATH set but JWT encode failed"
            ),
        }
    }

    let state = Arc::new(AppState {
        pool,
        jwt_secret: config.jwt_secret.clone(),
        server_key_encryption_key: config.server_key_encryption_key,
        demo_logins_public: config.demo_logins_public,
        allow_local_password_auth: config.allow_local_password_auth,
        stripe_webhook_secret: config.stripe_webhook_secret.clone(),
        adyen_hmac_key_hex: config.adyen_hmac_key_hex.clone(),
        cdn_purge_webhook_url: config.cdn_purge_webhook_url.clone(),
        deploywerk_git_sha: config.deploywerk_git_sha.clone(),
        oidc,
        scim_bearer_token,
        scim_idp_issuer,
        mollie_api_key: config.mollie_api_key.clone(),
        deploy_worker: config.deploy_worker_config(),
        deploy_dispatch_inline,
        github_app_webhook_secret: config.github_app_webhook_secret.clone(),
        github_app_slug: config.github_app_slug.clone(),
        idp_admin_url: config.resolved_idp_admin_url(),
        smtp_settings: config.smtp_settings.clone(),
        public_app_url: config.public_app_url.clone(),
        admin_action_emails_enabled: config.admin_action_emails_enabled,
        local_service_defaults: config.local_service_defaults,
        integration_urls: config.integration_urls.clone(),
        documentation_base_url: config.documentation_base_url.clone(),
        technitium_integration,
        portainer_integration,
    });

    if !deploy_dispatch_inline {
        tracing::warn!(
            "DEPLOYWERK_DEPLOY_DISPATCH=external: API will not run deploy jobs; start deploywerk-deploy-worker"
        );
    }

    let health_pool = state.pool.clone();
    tokio::spawn(async move {
        team_platform::run_health_check_loop(health_pool).await;
    });

    let housekeeping_pool = state.pool.clone();
    tokio::spawn(async move {
        team_platform::run_housekeeping_loop(housekeeping_pool).await;
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let concurrency_limit = std::env::var("HTTP_CONCURRENCY_LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(256usize);

    let app = Router::new()
        .merge(scim::routes())
        .merge(oidc::routes())
        .merge(mfa::routes())
        .merge(saml::routes())
        .merge(mail_platform::routes())
        .merge(handlers::routes())
        .layer(ConcurrencyLimitLayer::new(concurrency_limit))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(
                    DefaultMakeSpan::new()
                        .level(Level::INFO)
                        .include_headers(false),
                )
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(LatencyUnit::Micros),
                ),
        )
        .layer(cors)
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://{}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
