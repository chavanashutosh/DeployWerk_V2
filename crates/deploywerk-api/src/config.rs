use std::env;
use std::fs;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use sha2::{Digest, Sha256};

use crate::mail;

/// Public base URLs for operator integrations (Git, mail admin, Portainer, DNS UI, Matrix client, Traefik).
/// Set via `DEPLOYWERK_INTEGRATION_*` env vars; exposed in `GET /api/v1/bootstrap` (no secrets).
#[derive(Clone, Default, Debug)]
pub struct IntegrationUrls {
    pub forgejo_url: Option<String>,
    pub mailcow_url: Option<String>,
    pub portainer_url: Option<String>,
    pub technitium_url: Option<String>,
    pub matrix_client_url: Option<String>,
    pub traefik_dashboard_url: Option<String>,
}

fn optional_integration_url(var: &str) -> Option<String> {
    env::var(var)
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
}

/// Fills unset [`IntegrationUrls`] when `DEPLOYWERK_LOCAL_SERVICE_DEFAULTS` is enabled.
///
/// Defaults match common **published ports** when Traefik, Portainer, Forgejo, Technitium, and Mailcow
/// run on the same host as DeployWerk. Explicit `DEPLOYWERK_INTEGRATION_*` values always win.
///
/// | Slot | Default |
/// |------|---------|
/// | Traefik dashboard | `http://127.0.0.1:8080` |
/// | Portainer | `https://127.0.0.1:9443` (self-signed TLS; browser may warn) |
/// | Forgejo | `http://127.0.0.1:3000` |
/// | Technitium admin | `http://127.0.0.1:5380` |
/// | Mailcow web | `https://127.0.0.1:8444` |
/// | Matrix client | `http://127.0.0.1:8088` |
pub(crate) fn merge_local_integration_defaults(urls: &mut IntegrationUrls) {
    if urls.traefik_dashboard_url.is_none() {
        urls.traefik_dashboard_url = Some("http://127.0.0.1:8080".into());
    }
    if urls.portainer_url.is_none() {
        urls.portainer_url = Some("https://127.0.0.1:9443".into());
    }
    if urls.forgejo_url.is_none() {
        urls.forgejo_url = Some("http://127.0.0.1:3000".into());
    }
    if urls.technitium_url.is_none() {
        urls.technitium_url = Some("http://127.0.0.1:5380".into());
    }
    if urls.mailcow_url.is_none() {
        urls.mailcow_url = Some("https://127.0.0.1:8444".into());
    }
    if urls.matrix_client_url.is_none() {
        urls.matrix_client_url = Some("http://127.0.0.1:8088".into());
    }
}

/// Values needed by the background deploy worker (clone into spawned tasks).
#[derive(Clone)]
pub struct DeployWorkerConfig {
    pub server_key_encryption_key: [u8; 32],
    pub platform_docker_enabled: bool,
    pub apps_base_domain: Option<String>,
    pub git_cache_root: String,
    pub volumes_root: String,
    /// `none` or `traefik`
    pub edge_mode: String,
    pub traefik_docker_network: String,
    pub app_container_port: u16,
    /// When true, `pr_preview` jobs create a dedicated Docker network and attach the preview container.
    pub pr_preview_isolated_network: bool,
    /// Instance SMTP for deploy notification emails (same env as API).
    pub smtp_settings: Option<mail::SmtpSettings>,
}

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    /// 32-byte key for AES-256-GCM encryption of SSH private keys at rest.
    pub server_key_encryption_key: [u8; 32],
    pub host: String,
    pub port: u16,
    pub seed_demo_users: bool,
    pub demo_logins_public: bool,
    /// `whsec_…` signing secret from Stripe; when set, `/api/v1/stripe/webhook` verifies signatures.
    pub stripe_webhook_secret: Option<String>,
    /// Hex-encoded HMAC key from Adyen Customer Area; when set, `/api/v1/hooks/adyen` verifies `hmacSignature`.
    pub adyen_hmac_key_hex: Option<String>,
    /// Optional URL to `POST` JSON `{ team_id, purge_id, paths }` after recording a CDN purge request.
    pub cdn_purge_webhook_url: Option<String>,
    /// If set after migrations, that user receives `is_platform_admin` on startup (if the row exists).
    pub bootstrap_platform_admin_email: Option<String>,
    /// Shown in GET `/api/v1/admin/system` (optional; can also set `DEPLOYWERK_GIT_SHA` at compile time).
    pub deploywerk_git_sha: Option<String>,
    /// When false, `/api/v1/auth/register` and password login for SSO-only users are blocked.
    pub allow_local_password_auth: bool,
    /// OpenID Connect issuer URL (e.g. Authentik application issuer).
    pub authentik_issuer: Option<String>,
    pub authentik_client_id: Option<String>,
    pub authentik_client_secret: Option<String>,
    pub authentik_redirect_uri: Option<String>,
    /// Full Authentik admin UI URL (e.g. `http://127.0.0.1:9000/if/admin/`). Overrides derived URL.
    pub authentik_admin_url: Option<String>,
    /// Authentik origin without path (e.g. `http://127.0.0.1:9000`). Used with issuer to derive admin URL.
    pub authentik_browser_base_url: Option<String>,
    /// Bearer token for inbound SCIM (`Authorization: Bearer …`).
    pub deploywerk_scim_bearer_token: Option<String>,
    /// Issuer URL stored on `users.idp_issuer` for SCIM users; defaults to `AUTHENTIK_ISSUER`.
    pub deploywerk_scim_idp_issuer: Option<String>,
    /// Mollie API key (`live_` / `test_`); used to verify webhook payment ids.
    pub mollie_api_key: Option<String>,
    /// Run deploy jobs on the API host via local `docker` (see `DEPLOYWERK_PLATFORM_DOCKER_ENABLED`).
    pub platform_docker_enabled: bool,
    /// Base domain for auto-generated app hostnames (e.g. `apps.example.com`).
    pub apps_base_domain: Option<String>,
    /// `none` | `traefik` — extra `docker run` labels/network for edge routing.
    pub edge_mode: String,
    /// Docker network Traefik shares with app containers.
    pub traefik_docker_network: String,
    /// Container port apps listen on (for Traefik / published port hints).
    pub app_container_port: u16,
    /// When true (default), the API spawns deploy execution tasks in-process. When false, use `deploywerk-deploy-worker`.
    pub deploy_dispatch_inline: bool,
    /// HMAC secret for `POST /api/v1/hooks/github-app` (GitHub App webhooks).
    pub github_app_webhook_secret: Option<String>,
    /// For `GET .../github-app/install-url` — GitHub App slug in the install link.
    pub github_app_slug: Option<String>,
    /// GitHub App id (numeric) for JWT used with GitHub’s installation token API.
    pub github_app_id: Option<u64>,
    /// PEM read from `GITHUB_APP_PRIVATE_KEY_PATH` when set.
    pub github_app_private_key_pem: Option<String>,
    /// Parsed SMTP settings when `DEPLOYWERK_SMTP_HOST` and `DEPLOYWERK_SMTP_FROM` are set.
    pub smtp_settings: Option<mail::SmtpSettings>,
    /// Public web UI base URL (invite links in email), e.g. `https://app.example.com`.
    pub public_app_url: Option<String>,
    /// When true, send best-effort SMTP mail after sensitive super-admin mutations (requires SMTP).
    pub admin_action_emails_enabled: bool,
    /// When true, unset integration URLs get 127.0.0.1 presets. Set via `DEPLOYWERK_LOCAL_SERVICE_DEFAULTS`, or defaults on in `APP_ENV=development` when unset.
    pub local_service_defaults: bool,
    /// Links shown under Platform integrations (bootstrap + UI).
    pub integration_urls: IntegrationUrls,
    /// Optional base URL for operator-hosted docs (e.g. raw GitHub docs tree). SSO section link uses `{base}/README.md#single-sign-on-oidc` when set.
    pub documentation_base_url: Option<String>,
    /// Technitium DNS HTTP API automation (optional; feature-flagged).
    pub technitium_dns_enabled: bool,
    pub technitium_api_url: Option<String>,
    pub technitium_api_token: Option<String>,
    /// Portainer read-only probe (optional; default off).
    pub portainer_integration_enabled: bool,
    pub portainer_api_url: Option<String>,
    pub portainer_api_token: Option<String>,
}

fn browser_origin_from_issuer(iss: &str) -> Option<&str> {
    let iss = iss.trim();
    let pos = iss.find("://")?;
    let rest = &iss[pos + 3..];
    let end = rest.find('/').unwrap_or(rest.len());
    let authority = rest.get(..end)?;
    if authority.is_empty() {
        return None;
    }
    iss.get(..pos + 3 + end)
}

fn parse_32_byte_key(raw: &str) -> Option<[u8; 32]> {
    let t = raw.trim();
    if t.len() == 64 && t.chars().all(|c| c.is_ascii_hexdigit()) {
        let mut out = [0u8; 32];
        for (i, chunk) in t.as_bytes().chunks(2).enumerate() {
            let s = std::str::from_utf8(chunk).ok()?;
            out[i] = u8::from_str_radix(s, 16).ok()?;
        }
        return Some(out);
    }
    let dec = STANDARD.decode(t).ok()?;
    if dec.len() == 32 {
        let mut out = [0u8; 32];
        out.copy_from_slice(&dec);
        return Some(out);
    }
    None
}

fn derive_dev_server_key(jwt_secret: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"deploywerk-server-key-v1");
    h.update(jwt_secret.as_bytes());
    h.finalize().into()
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let app_env = env::var("APP_ENV").unwrap_or_else(|_| "development".into());
        let is_production = app_env.eq_ignore_ascii_case("production");

        let seed_demo_users = env::var("SEED_DEMO_USERS")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(!is_production);

        let demo_logins_public = env::var("DEMO_LOGINS_PUBLIC")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(!is_production);

        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
            tracing::warn!("JWT_SECRET not set; using insecure dev default");
            "dev-insecure-change-me".into()
        });

        let server_key_encryption_key = match env::var("SERVER_KEY_ENCRYPTION_KEY") {
            Ok(s) => match parse_32_byte_key(&s) {
                Some(k) => k,
                None => {
                    if is_production {
                        panic!("SERVER_KEY_ENCRYPTION_KEY must be 32 bytes (64 hex chars or base64)");
                    }
                    tracing::warn!(
                        "SERVER_KEY_ENCRYPTION_KEY invalid or missing; deriving from JWT_SECRET (dev only)"
                    );
                    derive_dev_server_key(&jwt_secret)
                }
            },
            Err(_) => {
                if is_production {
                    panic!("SERVER_KEY_ENCRYPTION_KEY is required in production");
                }
                tracing::warn!(
                    "SERVER_KEY_ENCRYPTION_KEY not set; deriving from JWT_SECRET (dev only)"
                );
                derive_dev_server_key(&jwt_secret)
            }
        };

        let stripe_webhook_secret = env::var("STRIPE_WEBHOOK_SECRET")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let adyen_hmac_key_hex = env::var("ADYEN_HMAC_KEY_HEX")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let cdn_purge_webhook_url = env::var("DEPLOYWERK_CDN_PURGE_WEBHOOK_URL")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let bootstrap_platform_admin_email = env::var("DEPLOYWERK_BOOTSTRAP_PLATFORM_ADMIN_EMAIL")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let deploywerk_git_sha = env::var("DEPLOYWERK_GIT_SHA")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let allow_local_password_auth = env::var("ALLOW_LOCAL_PASSWORD_AUTH")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(!is_production);

        let authentik_issuer = env::var("AUTHENTIK_ISSUER")
            .ok()
            .map(|s| s.trim().trim_end_matches('/').to_string())
            .filter(|s| !s.is_empty());
        let authentik_client_id = env::var("AUTHENTIK_CLIENT_ID")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let authentik_client_secret = env::var("AUTHENTIK_CLIENT_SECRET")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let authentik_redirect_uri = env::var("AUTHENTIK_REDIRECT_URI")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let authentik_admin_url = env::var("AUTHENTIK_ADMIN_URL")
            .ok()
            .map(|s| s.trim().trim_end_matches('/').to_string())
            .filter(|s| !s.is_empty());

        let authentik_browser_base_url = env::var("AUTHENTIK_BROWSER_BASE_URL")
            .ok()
            .map(|s| s.trim().trim_end_matches('/').to_string())
            .filter(|s| !s.is_empty());

        let deploywerk_scim_bearer_token = env::var("DEPLOYWERK_SCIM_BEARER_TOKEN")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let deploywerk_scim_idp_issuer = env::var("DEPLOYWERK_SCIM_IDP_ISSUER")
            .ok()
            .map(|s| s.trim().trim_end_matches('/').to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| authentik_issuer.clone());

        let mollie_api_key = env::var("MOLLIE_API_KEY")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let platform_docker_enabled = env::var("DEPLOYWERK_PLATFORM_DOCKER_ENABLED")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let apps_base_domain = env::var("DEPLOYWERK_APPS_BASE_DOMAIN")
            .ok()
            .map(|s| s.trim().trim_end_matches('.').to_lowercase())
            .filter(|s| !s.is_empty());

        let edge_mode = env::var("DEPLOYWERK_EDGE_MODE")
            .unwrap_or_else(|_| "none".into())
            .trim()
            .to_lowercase();
        let edge_mode = if edge_mode.is_empty() {
            "none".into()
        } else {
            edge_mode
        };

        let traefik_docker_network = env::var("DEPLOYWERK_TRAEFIK_DOCKER_NETWORK")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "traefik".into());

        let app_container_port = env::var("DEPLOYWERK_APP_CONTAINER_PORT")
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(8080u16);

        let dispatch_raw = env::var("DEPLOYWERK_DEPLOY_DISPATCH").unwrap_or_else(|_| "inline".into());
        let d = dispatch_raw.trim().to_lowercase();
        let deploy_dispatch_inline = !matches!(d.as_str(), "external" | "worker" | "queue");

        let github_app_webhook_secret = env::var("GITHUB_APP_WEBHOOK_SECRET")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let github_app_slug = env::var("GITHUB_APP_SLUG")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let github_app_id = env::var("GITHUB_APP_ID")
            .ok()
            .and_then(|s| s.trim().parse().ok());

        let github_app_private_key_pem = env::var("GITHUB_APP_PRIVATE_KEY_PATH")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .and_then(|path| fs::read_to_string(&path).ok())
            .and_then(|pem| {
                let t = pem.trim();
                if t.is_empty() {
                    None
                } else {
                    Some(t.to_string())
                }
            });

        let smtp_host = env::var("DEPLOYWERK_SMTP_HOST")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let smtp_port = env::var("DEPLOYWERK_SMTP_PORT")
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(587u16);
        let smtp_user = env::var("DEPLOYWERK_SMTP_USER")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let smtp_password = env::var("DEPLOYWERK_SMTP_PASSWORD")
            .ok()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());
        let smtp_from = env::var("DEPLOYWERK_SMTP_FROM")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let smtp_tls = env::var("DEPLOYWERK_SMTP_TLS")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let smtp_settings = mail::parse_smtp_settings(
            smtp_host,
            smtp_port,
            smtp_user,
            smtp_password,
            smtp_from,
            smtp_tls,
        );

        let public_app_url = env::var("DEPLOYWERK_PUBLIC_APP_URL")
            .ok()
            .map(|s| s.trim().trim_end_matches('/').to_string())
            .filter(|s| !s.is_empty());

        let admin_action_emails_enabled = env::var("DEPLOYWERK_ADMIN_ACTION_EMAILS")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let mut integration_urls = IntegrationUrls {
            forgejo_url: optional_integration_url("DEPLOYWERK_INTEGRATION_FORGEJO_URL"),
            mailcow_url: optional_integration_url("DEPLOYWERK_INTEGRATION_MAILCOW_URL"),
            portainer_url: optional_integration_url("DEPLOYWERK_INTEGRATION_PORTAINER_URL"),
            technitium_url: optional_integration_url("DEPLOYWERK_INTEGRATION_TECHNITIUM_URL"),
            matrix_client_url: optional_integration_url("DEPLOYWERK_INTEGRATION_MATRIX_CLIENT_URL"),
            traefik_dashboard_url: optional_integration_url("DEPLOYWERK_INTEGRATION_TRAEFIK_URL"),
        };

        let local_service_defaults = match env::var("DEPLOYWERK_LOCAL_SERVICE_DEFAULTS") {
            Ok(v) if v == "1" || v.eq_ignore_ascii_case("true") => true,
            Ok(v) if v == "0" || v.eq_ignore_ascii_case("false") => false,
            Ok(_) => false,
            Err(_) => app_env.eq_ignore_ascii_case("development"),
        };
        if local_service_defaults {
            merge_local_integration_defaults(&mut integration_urls);
        }

        let documentation_base_url = optional_integration_url("DEPLOYWERK_DOCUMENTATION_BASE_URL");

        let technitium_dns_enabled = env::var("DEPLOYWERK_TECHNITIUM_DNS_ENABLED")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let technitium_api_url = optional_integration_url("DEPLOYWERK_TECHNITIUM_API_URL");
        let technitium_api_token = env::var("DEPLOYWERK_TECHNITIUM_API_TOKEN")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let portainer_integration_enabled = env::var("DEPLOYWERK_PORTAINER_INTEGRATION_ENABLED")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let portainer_api_url = optional_integration_url("DEPLOYWERK_PORTAINER_API_URL");
        let portainer_api_token = env::var("DEPLOYWERK_PORTAINER_API_TOKEN")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        Self {
            database_url: {
                #[cfg(feature = "postgres")]
                {
                    env::var("DATABASE_URL").unwrap_or_else(|_| {
                        "postgresql://deploywerk:deploywerk@127.0.0.1:5432/deploywerk".into()
                    })
                }
                #[cfg(feature = "sqlite")]
                {
                    env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://./data/deploywerk.db".into())
                }
            },
            jwt_secret,
            server_key_encryption_key,
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            // When unset, defaults above keep demos off in production; explicit SEED_DEMO_USERS / DEMO_LOGINS_PUBLIC
            // may be enabled in production (sensitive — demo accounts and public passwords on bootstrap).
            seed_demo_users,
            demo_logins_public,
            stripe_webhook_secret,
            adyen_hmac_key_hex,
            cdn_purge_webhook_url,
            bootstrap_platform_admin_email,
            deploywerk_git_sha,
            allow_local_password_auth,
            authentik_issuer,
            authentik_client_id,
            authentik_client_secret,
            authentik_redirect_uri,
            authentik_admin_url,
            authentik_browser_base_url,
            deploywerk_scim_bearer_token,
            deploywerk_scim_idp_issuer,
            mollie_api_key,
            platform_docker_enabled,
            apps_base_domain,
            edge_mode,
            traefik_docker_network,
            app_container_port,
            deploy_dispatch_inline,
            github_app_webhook_secret,
            github_app_slug,
            github_app_id,
            github_app_private_key_pem,
            smtp_settings,
            public_app_url,
            admin_action_emails_enabled,
            local_service_defaults,
            integration_urls,
            documentation_base_url,
            technitium_dns_enabled,
            technitium_api_url,
            technitium_api_token,
            portainer_integration_enabled,
            portainer_api_url,
            portainer_api_token,
        }
    }

    /// Link to Authentik’s admin UI for operators (bootstrap + login page).
    pub fn resolved_idp_admin_url(&self) -> Option<String> {
        if let Some(ref u) = self.authentik_admin_url {
            let t = u.trim();
            if !t.is_empty() {
                return Some(format!("{}/", t.trim_end_matches('/')));
            }
        }
        let base = self
            .authentik_browser_base_url
            .as_ref()
            .map(|s| s.as_str())
            .or_else(|| {
                self.authentik_issuer
                    .as_ref()
                    .map(|s| s.as_str())
                    .and_then(browser_origin_from_issuer)
            })?;
        let b = base.trim().trim_end_matches('/');
        if b.is_empty() {
            return None;
        }
        Some(format!("{b}/if/admin/"))
    }

    pub fn deploy_worker_config(&self) -> DeployWorkerConfig {
        let pr_preview_isolated_network = env::var("DEPLOYWERK_PR_PREVIEW_ISOLATED_NETWORK")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let git_cache_root = env::var("DEPLOYWERK_GIT_CACHE_ROOT")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "/var/lib/deploywerk/git-cache".into());

        let volumes_root = env::var("DEPLOYWERK_VOLUMES_ROOT")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "/var/lib/deploywerk/volumes".into());

        DeployWorkerConfig {
            server_key_encryption_key: self.server_key_encryption_key,
            platform_docker_enabled: self.platform_docker_enabled,
            apps_base_domain: self.apps_base_domain.clone(),
            git_cache_root,
            volumes_root,
            edge_mode: self.edge_mode.clone(),
            traefik_docker_network: self.traefik_docker_network.clone(),
            app_container_port: self.app_container_port,
            pr_preview_isolated_network,
            smtp_settings: self.smtp_settings.clone(),
        }
    }
}

#[cfg(test)]
mod integration_defaults_tests {
    use super::{merge_local_integration_defaults, IntegrationUrls};

    #[test]
    fn merge_local_defaults_fills_empty_slots() {
        let mut u = IntegrationUrls::default();
        merge_local_integration_defaults(&mut u);
        assert_eq!(u.traefik_dashboard_url.as_deref(), Some("http://127.0.0.1:8080"));
        assert_eq!(u.portainer_url.as_deref(), Some("https://127.0.0.1:9443"));
        assert_eq!(u.forgejo_url.as_deref(), Some("http://127.0.0.1:3000"));
        assert_eq!(u.technitium_url.as_deref(), Some("http://127.0.0.1:5380"));
        assert_eq!(u.mailcow_url.as_deref(), Some("https://127.0.0.1:8444"));
        assert!(u.matrix_client_url.is_none());
    }

    #[test]
    fn merge_local_defaults_preserves_explicit_urls() {
        let mut u = IntegrationUrls {
            forgejo_url: Some("http://custom:9000".into()),
            ..Default::default()
        };
        merge_local_integration_defaults(&mut u);
        assert_eq!(u.forgejo_url.as_deref(), Some("http://custom:9000"));
        assert_eq!(u.portainer_url.as_deref(), Some("https://127.0.0.1:9443"));
    }
}
