//! Polls PostgreSQL for `queued` deploy jobs and executes them (use with `DEPLOYWERK_DEPLOY_DISPATCH=external` on the API).
//!
//! Environment: same `DATABASE_URL`, `SERVER_KEY_ENCRYPTION_KEY`, and deploy-related vars as the API (`DEPLOYWERK_PLATFORM_*`, `DEPLOYWERK_EDGE_*`, etc.). Does **not** run migrations — start the API once to migrate.

use std::time::Duration;

use deploywerk_api::{execute_deploy_job, try_claim_next_queued_deploy_job, Config};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "deploywerk_api=info".into()))
        .init();

    let config = Config::from_env();
    let poll_ms: u64 = std::env::var("DEPLOYWERK_WORKER_POLL_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2000);

    let pool = PgPoolOptions::new()
        .max_connections(
            std::env::var("DATABASE_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
        )
        .connect(&config.database_url)
        .await?;

    let worker_cfg = config.deploy_worker_config();
    tracing::info!(poll_ms, "deploy worker started (no HTTP server)");

    loop {
        match try_claim_next_queued_deploy_job(&pool).await {
            Ok(Some((job_id, application_id))) => {
                tracing::info!(%job_id, %application_id, "claimed deploy job");
                let pool_clone = pool.clone();
                let cfg_clone = worker_cfg.clone();
                execute_deploy_job(pool_clone, cfg_clone, job_id, application_id).await;
                tracing::info!(%job_id, "deploy job finished");
            }
            Ok(None) => {
                tokio::time::sleep(Duration::from_millis(poll_ms)).await;
            }
            Err(e) => {
                tracing::error!(?e, "claim deploy job failed");
                tokio::time::sleep(Duration::from_millis(poll_ms)).await;
            }
        }
    }
}
