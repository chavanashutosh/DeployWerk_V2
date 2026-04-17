//! Lightweight host agent: periodic heartbeat to the DeployWerk API.
//!
//! Environment:
//! - `DEPLOYWERK_API_URL` — base URL, e.g. `http://localhost:8080` (match API `HOST`/`PORT`)
//! - `DEPLOYWERK_AGENT_TOKEN` — bearer token from team **Agent** registration

use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "deploywerk_agent=info".into()),
        )
        .init();

    let base = env::var("DEPLOYWERK_API_URL").unwrap_or_else(|_| "http://localhost:8080".into());
    let token = env::var("DEPLOYWERK_AGENT_TOKEN").map_err(|_| {
        anyhow::anyhow!("DEPLOYWERK_AGENT_TOKEN is required (register an agent in the DeployWerk UI)")
    })?;

    let version = env!("CARGO_PKG_VERSION");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?;

    let url = format!(
        "{}/api/v1/agent/heartbeat",
        base.trim_end_matches('/')
    );

    let mut ticker = tokio::time::interval(Duration::from_secs(60));
    loop {
        ticker.tick().await;
        let body = serde_json::json!({
            "version": format!("deploywerk-agent/{version}"),
            "meta": { "os": std::env::consts::OS }
        });
        let res = client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .json(&body)
            .send()
            .await;

        match res {
            Ok(r) if r.status().is_success() => tracing::info!("heartbeat ok"),
            Ok(r) => tracing::warn!(status = %r.status(), "heartbeat failed"),
            Err(e) => tracing::warn!(?e, "heartbeat error"),
        }
    }
}
