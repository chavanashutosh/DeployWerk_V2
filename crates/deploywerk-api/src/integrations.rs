//! Optional operator integrations (Technitium DNS API, Portainer health probe).

#[derive(Clone)]
#[allow(dead_code)]
pub struct TechnitiumIntegration {
    pub api_url: String,
    pub api_token: String,
}

#[derive(Clone)]
pub struct PortainerIntegration {
    pub api_url: String,
    pub api_token: String,
}

/// GET `{base}/api/system/status` with `X-API-Key` (Portainer CE 2.x).
pub async fn portainer_system_status(
    p: &PortainerIntegration,
) -> Result<serde_json::Value, String> {
    let base = p.api_url.trim().trim_end_matches('/');
    let url = format!("{base}/api/system/status");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|e| e.to_string())?;
    let res = client
        .get(&url)
        .header("X-API-Key", &p.api_token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("Portainer HTTP {}", res.status()));
    }
    res.json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())
}

