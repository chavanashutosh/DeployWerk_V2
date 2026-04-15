//! Applications under environments, deploy jobs, and background SSH deploy worker.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use deploywerk_core::{
    ApplicationDetail, ApplicationEnvVarPublic, ApplicationSummary, DeployJobStatus,
    DeployJobSummary, RuntimeVolumeMount,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use hmac::{Hmac, Mac};
use sha2::Digest;
use sha2::Sha256;

use crate::auth::require_principal;
use crate::config::DeployWorkerConfig;
use crate::crypto_keys::decrypt_private_key;
use crate::deploy_schedule::deploy_schedule_allows_now;
use crate::error::ApiError;
use crate::rbac::{
    application_ids_for_user_in_environment,
    require_application_deploy,
    require_application_mutate,
    require_application_read,
    require_some_app_membership_in_environment,
    require_team_access_read,
    require_team_access_mutate,
    user_can_see_application_secrets,
};
use crate::notifications::spawn_deploy_notifications;
use crate::servers::{run_local_sh, run_ssh_exec};
use crate::audit::try_log_team_audit;
use crate::AppState;

/// Lowercase path for matching GitLab `path_with_namespace` (and generic remotes) to `git_repo_full_name`.
pub fn normalize_git_remote_path(raw: &str) -> String {
    raw.trim()
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_lowercase()
}

/// Normalize user/repo or GitHub URL to lowercase `owner/repo`.
pub fn normalize_github_repo_full_name(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    let s = s.strip_prefix("https://").unwrap_or(s);
    let s = s.strip_prefix("http://").unwrap_or(s);
    let s = s.strip_prefix("www.").unwrap_or(s);
    let s = s.strip_prefix("github.com/").unwrap_or(s);
    let s = s.trim_end_matches('/').trim_end_matches(".git");
    let parts: Vec<&str> = s.split('/').filter(|x| !x.is_empty()).collect();
    if parts.len() >= 2 {
        let owner = parts[parts.len() - 2].to_lowercase();
        let repo = parts[parts.len() - 1].to_lowercase();
        Some(format!("{owner}/{repo}"))
    } else {
        None
    }
}

/// `*` matches any branch; exact name; or `release/*` style prefix.
pub fn branch_matches_git_pattern(pattern: &str, branch: &str) -> bool {
    let p = pattern.trim();
    if p.is_empty() || p == "*" || p == "**" {
        return true;
    }
    if let Some(prefix) = p.strip_suffix("/*") {
        return branch == prefix || branch.starts_with(&format!("{prefix}/"));
    }
    p == branch
}

/// Enqueue deploy from GitHub webhook (no JWT). Verifies application belongs to `expected_team_id`.
pub async fn enqueue_deploy_for_webhook(
    state: Arc<AppState>,
    application_id: Uuid,
    expected_team_id: Uuid,
    git_ref: String,
    git_sha: String,
) -> Result<EnqueueDeployResponse, ApiError> {
    let tid: Option<Uuid> = sqlx::query_scalar(
        r#"SELECT p.team_id FROM applications a
           JOIN environments e ON e.id = a.environment_id
           JOIN projects p ON p.id = e.project_id
           WHERE a.id = $1"#,
    )
    .bind(application_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    if tid != Some(expected_team_id) {
        return Err(ApiError::Forbidden);
    }

    enqueue_deploy_inner(
        state,
        application_id,
        Some((git_ref, git_sha)),
        None,
        "standard",
        None,
        false,
    )
    .await
}

/// PR preview deploy (GitHub App webhook). Same queue as standard deploys.
pub async fn enqueue_pr_preview_deploy(
    state: Arc<AppState>,
    application_id: Uuid,
    expected_team_id: Uuid,
    git_ref: String,
    git_sha: String,
    git_base_sha: Option<String>,
    pr_number: i32,
) -> Result<EnqueueDeployResponse, ApiError> {
    let tid: Option<Uuid> = sqlx::query_scalar(
        r#"SELECT p.team_id FROM applications a
           JOIN environments e ON e.id = a.environment_id
           JOIN projects p ON p.id = e.project_id
           WHERE a.id = $1"#,
    )
    .bind(application_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    if tid != Some(expected_team_id) {
        return Err(ApiError::Forbidden);
    }

    enqueue_deploy_inner(
        state,
        application_id,
        Some((git_ref, git_sha)),
        git_base_sha,
        "pr_preview",
        Some(pr_number),
        false,
    )
    .await
}

/// Tear down a PR preview container (no image pull / run).
pub async fn enqueue_pr_preview_destroy(
    state: Arc<AppState>,
    application_id: Uuid,
    expected_team_id: Uuid,
    pr_number: i32,
) -> Result<EnqueueDeployResponse, ApiError> {
    let tid: Option<Uuid> = sqlx::query_scalar(
        r#"SELECT p.team_id FROM applications a
           JOIN environments e ON e.id = a.environment_id
           JOIN projects p ON p.id = e.project_id
           WHERE a.id = $1"#,
    )
    .bind(application_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    if tid != Some(expected_team_id) {
        return Err(ApiError::Forbidden);
    }

    enqueue_deploy_inner(
        state,
        application_id,
        None,
        None,
        "pr_preview_destroy",
        Some(pr_number),
        false,
    )
    .await
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/v1/teams/{team_id}/deployments",
            get(list_team_deployments),
        )
        .route("/api/v1/teams/{team_id}/domains", get(list_team_domains))
        .route(
            "/api/v1/teams/{team_id}/projects/{project_id}/environments/{environment_id}/applications",
            get(list_applications).post(create_application),
        )
        .route(
            "/api/v1/teams/{team_id}/projects/{project_id}/environments/{environment_id}/applications/{application_id}",
            get(get_application)
                .patch(update_application)
                .delete(delete_application),
        )
        .route(
            "/api/v1/teams/{team_id}/projects/{project_id}/environments/{environment_id}/applications/{application_id}/deploy",
            post(enqueue_deploy_scoped),
        )
        .route(
            "/api/v1/teams/{team_id}/projects/{project_id}/environments/{environment_id}/applications/{application_id}/rollback",
            post(enqueue_rollback_scoped),
        )
        .route(
            "/api/v1/teams/{team_id}/projects/{project_id}/environments/{environment_id}/applications/{application_id}/container-log-stream",
            get(stream_application_container_log),
        )
        .route(
            "/api/v1/teams/{team_id}/projects/{project_id}/environments/{environment_id}/applications/{application_id}/deploy-jobs",
            get(list_deploy_jobs),
        )
        .route(
            "/api/v1/teams/{team_id}/deploy-jobs/{job_id}/log-stream",
            get(stream_deploy_job_log),
        )
        .route(
            "/api/v1/teams/{team_id}/deploy-jobs/{job_id}",
            get(get_deploy_job),
        )
        .route(
            "/api/v1/teams/{team_id}/deploy-jobs/{job_id}/approve",
            post(approve_deploy_job),
        )
}

#[derive(Serialize)]
pub struct EnqueueDeployResponse {
    pub job_id: Uuid,
    pub status: DeployJobStatus,
}

#[derive(FromRow)]
struct TeamDeploymentListRow {
    job_id: Uuid,
    application_id: Uuid,
    application_name: String,
    application_slug: String,
    environment_id: Uuid,
    environment_name: String,
    project_id: Uuid,
    project_name: String,
    status: String,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    git_ref: Option<String>,
    git_sha: Option<String>,
    job_kind: String,
    pr_number: Option<i32>,
    git_repo_full_name: Option<String>,
    git_base_sha: Option<String>,
    auto_hostname: Option<String>,
    domains: serde_json::Value,
    deploy_strategy: String,
}

#[derive(Serialize)]
pub struct TeamDeploymentRow {
    pub job_id: Uuid,
    pub application_id: Uuid,
    pub application_name: String,
    pub application_slug: String,
    pub environment_id: Uuid,
    pub environment_name: String,
    pub project_id: Uuid,
    pub project_name: String,
    pub status: DeployJobStatus,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_repo_full_name: Option<String>,
    /// Preferred HTTPS URL (auto hostname or first custom domain).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_url: Option<String>,
    /// GitHub commit page when `git_repo_full_name` is `owner/repo` and `git_sha` is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_commit_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_base_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_compare_url: Option<String>,
    /// Effective strategy: job override, else application default (`standard` if unset).
    #[serde(default, skip_serializing_if = "is_standard_strategy")]
    pub deploy_strategy: Option<String>,
}

fn is_standard_strategy(s: &Option<String>) -> bool {
    s.as_deref().map(str::trim).unwrap_or("standard") == "standard"
}

fn primary_public_url(auto_hostname: Option<&str>, domains_json: &serde_json::Value) -> Option<String> {
    if let Some(h) = auto_hostname.map(str::trim).filter(|s| !s.is_empty()) {
        return Some(format!("https://{h}"));
    }
    parse_domains(domains_json).into_iter().find_map(|d| {
        let d = d.trim();
        if d.is_empty() {
            return None;
        }
        if d.starts_with("http://") || d.starts_with("https://") {
            Some(d.to_string())
        } else {
            Some(format!("https://{d}"))
        }
    })
}

/// Deep link: `owner/repo` → GitHub; `group/sub/.../project` → gitlab.com (self‑hosted GitLab needs manual links).
fn source_commit_page_url(full_name: Option<&str>, sha: Option<&str>) -> Option<String> {
    let name = full_name?.trim();
    let sh = sha?.trim();
    if name.is_empty() || sh.len() < 7 || name.contains(' ') || name.starts_with("http") {
        return None;
    }
    match name.matches('/').count() {
        1 => Some(format!("https://github.com/{name}/commit/{sh}")),
        n if n >= 2 => Some(format!("https://gitlab.com/{name}/-/commit/{sh}")),
        _ => None,
    }
}

/// GitHub / GitLab compare URL when base + head SHAs exist (e.g. PR previews).
fn source_compare_url(full_name: Option<&str>, base: Option<&str>, head: Option<&str>) -> Option<String> {
    let name = full_name?.trim();
    let b = base?.trim();
    let h = head?.trim();
    if name.is_empty()
        || b.len() < 7
        || h.len() < 7
        || name.contains(' ')
        || name.starts_with("http")
    {
        return None;
    }
    match name.matches('/').count() {
        1 => Some(format!("https://github.com/{name}/compare/{b}...{h}")),
        n if n >= 2 => Some(format!("https://gitlab.com/{name}/-/compare/{b}...{h}")),
        _ => None,
    }
}

#[derive(Deserialize)]
struct ListDeploymentsQuery {
    #[serde(default = "default_deployments_limit")]
    limit: i64,
}

fn default_deployments_limit() -> i64 {
    50
}

async fn list_team_deployments(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    Query(q): Query<ListDeploymentsQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamDeploymentRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_access_read(&state.pool, p.user_id, team_id).await?;

    let limit = q.limit.clamp(1, 100);

    let rows: Vec<TeamDeploymentListRow> = sqlx::query_as(
        r#"SELECT j.id AS job_id, a.id AS application_id, a.name AS application_name, a.slug AS application_slug,
                  e.id AS environment_id, e.name AS environment_name, p.id AS project_id, p.name AS project_name,
                  j.status, j.created_at, j.started_at, j.finished_at,
                  j.git_ref, j.git_sha, j.git_base_sha, COALESCE(j.job_kind, 'standard') AS job_kind, j.pr_number,
                  a.git_repo_full_name, a.auto_hostname, COALESCE(a.domains, '[]'::jsonb) AS domains,
                  COALESCE(NULLIF(trim(j.deploy_strategy), ''), NULLIF(trim(a.deploy_strategy), ''), 'standard') AS deploy_strategy
           FROM deploy_jobs j
           JOIN applications a ON a.id = j.application_id
           JOIN environments e ON e.id = a.environment_id
           JOIN projects p ON p.id = e.project_id
           WHERE p.team_id = $1
           ORDER BY j.created_at DESC
           LIMIT $2"#,
    )
    .bind(team_id)
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let mut out = Vec::new();
    for r in rows {
        let primary_url = primary_public_url(r.auto_hostname.as_deref(), &r.domains);
        let source_commit_url =
            source_commit_page_url(r.git_repo_full_name.as_deref(), r.git_sha.as_deref());
        let source_compare_url = source_compare_url(
            r.git_repo_full_name.as_deref(),
            r.git_base_sha.as_deref(),
            r.git_sha.as_deref(),
        );
        let jk = r.job_kind.trim();
        let job_kind_opt = if jk == "standard" {
            None
        } else {
            Some(jk.to_string())
        };
        let ds = r.deploy_strategy.trim();
        let deploy_strategy_opt = if ds.is_empty() || ds == "standard" {
            None
        } else {
            Some(ds.to_string())
        };
        out.push(TeamDeploymentRow {
            job_id: r.job_id,
            application_id: r.application_id,
            application_name: r.application_name,
            application_slug: r.application_slug,
            environment_id: r.environment_id,
            environment_name: r.environment_name,
            project_id: r.project_id,
            project_name: r.project_name,
            status: job_status_from_db(&r.status)?,
            created_at: r.created_at,
            started_at: r.started_at,
            finished_at: r.finished_at,
            git_ref: r.git_ref,
            git_sha: r.git_sha,
            job_kind: job_kind_opt,
            pr_number: r.pr_number,
            git_repo_full_name: r.git_repo_full_name,
            primary_url,
            source_commit_url,
            git_base_sha: r.git_base_sha,
            source_compare_url,
            deploy_strategy: deploy_strategy_opt,
        });
    }
    Ok(Json(out))
}

#[derive(Serialize)]
pub struct TeamDomainRow {
    pub domain: String,
    pub application_id: Uuid,
    pub application_name: String,
    pub environment_name: String,
    pub project_name: String,
    /// True when this hostname matches the operator-provisioned `auto_hostname`.
    #[serde(default)]
    pub provisioned: bool,
}

async fn list_team_domains(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamDomainRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_access_read(&state.pool, p.user_id, team_id).await?;

    let rows: Vec<(String, Uuid, String, String, String, Option<String>)> = sqlx::query_as(
        r#"SELECT d.domain_name, a.id, a.name, e.name, p.name, a.auto_hostname
           FROM applications a
           JOIN environments e ON e.id = a.environment_id
           JOIN projects p ON p.id = e.project_id
           CROSS JOIN LATERAL jsonb_array_elements_text(COALESCE(a.domains, '[]'::jsonb))
             AS d(domain_name)
           WHERE p.team_id = $1
           ORDER BY d.domain_name, a.name"#,
    )
    .bind(team_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let out = rows
        .into_iter()
        .map(
            |(domain, application_id, application_name, environment_name, project_name, auto_h)| {
                TeamDomainRow {
                    provisioned: auto_h.as_deref() == Some(domain.as_str()),
                    domain,
                    application_id,
                    application_name,
                    environment_name,
                    project_name,
                }
            },
        )
        .collect();

    Ok(Json(out))
}

async fn ensure_env_in_team_project(
    pool: &sqlx::PgPool,
    team_id: Uuid,
    project_id: Uuid,
    environment_id: Uuid,
) -> Result<(), ApiError> {
    let ok: bool = sqlx::query_scalar(
        r#"SELECT EXISTS(
            SELECT 1 FROM environments e
            JOIN projects p ON p.id = e.project_id
            WHERE e.id = $1 AND e.project_id = $2 AND p.team_id = $3
        )"#,
    )
    .bind(environment_id)
    .bind(project_id)
    .bind(team_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    if ok {
        Ok(())
    } else {
        Err(ApiError::NotFound)
    }
}

async fn ensure_application_in_env(
    pool: &sqlx::PgPool,
    application_id: Uuid,
    environment_id: Uuid,
) -> Result<(), ApiError> {
    let ok: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM applications WHERE id = $1 AND environment_id = $2)",
    )
    .bind(application_id)
    .bind(environment_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    if ok {
        Ok(())
    } else {
        Err(ApiError::NotFound)
    }
}

fn job_status_from_db(s: &str) -> Result<DeployJobStatus, ApiError> {
    DeployJobStatus::parse(s).ok_or(ApiError::Internal)
}

fn docker_name_part(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .take(32)
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn docker_env_name_valid(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Caps env pairs passed to `docker run` (CLI length and operator safety).
const MAX_DOCKER_ENV_PAIRS: usize = 128;
const MAX_DOCKER_ENV_APPROX_BYTES: usize = 65536;

fn validate_env_for_docker_run(rows: &[(String, String, bool)]) -> Result<(), String> {
    let mut count = 0usize;
    let mut approx = 0usize;
    for (k, v, _) in rows {
        if !docker_env_name_valid(k) {
            continue;
        }
        count += 1;
        approx = approx.saturating_add(k.len().saturating_add(v.len()).saturating_add(16));
    }
    if count > MAX_DOCKER_ENV_PAIRS {
        return Err(format!(
            "too many application env vars for docker run after key validation ({count} > {MAX_DOCKER_ENV_PAIRS})"
        ));
    }
    if approx > MAX_DOCKER_ENV_APPROX_BYTES {
        return Err(format!(
            "application env vars too large for docker run (approx {approx} > {MAX_DOCKER_ENV_APPROX_BYTES} bytes)"
        ));
    }
    Ok(())
}

fn validate_and_normalize_runtime_volumes(
    vols: Vec<RuntimeVolumeMount>,
) -> Result<Vec<RuntimeVolumeMount>, ApiError> {
    let mut out: Vec<RuntimeVolumeMount> = Vec::new();
    for v in vols {
        let name = v.name.trim().to_string();
        let container_path = v.container_path.trim().to_string();
        if name.is_empty() || container_path.is_empty() {
            continue;
        }
        if name.len() > 64
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ApiError::BadRequest(
                "runtime volume name must be <= 64 chars and only [a-zA-Z0-9_-]".into(),
            ));
        }
        if !container_path.starts_with('/')
            || container_path.contains(' ')
            || container_path.len() > 256
        {
            return Err(ApiError::BadRequest(
                "runtime volume container_path must be an absolute path without spaces".into(),
            ));
        }
        out.push(RuntimeVolumeMount { name, container_path });
    }
    Ok(out)
}

pub(crate) fn preview_docker_container_name(slug: &str, application_id: Uuid, pr_number: i32) -> String {
    let short = application_id.as_simple().to_string();
    let short8 = &short[..short.len().min(8)];
    let slug_part = docker_name_part(slug);
    if slug_part.is_empty() {
        format!("dw-pr{pr_number}-{short8}")
    } else {
        format!("dw-pr{pr_number}-{slug_part}-{short8}")
    }
}

fn preview_docker_network_name(slug: &str, application_id: Uuid, pr_number: i32) -> String {
    let raw = preview_docker_container_name(slug, application_id, pr_number);
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn preview_traefik_hostname(base_domain: &str, pr_number: i32, slug: &str) -> String {
    let slug_part = docker_name_part(slug);
    let sp = if slug_part.is_empty() {
        "app".to_string()
    } else {
        slug_part
    };
    format!("pr{pr_number}-{sp}.{base_domain}")
}

#[derive(Clone)]
enum DeployExec {
    Ssh {
        host: String,
        port: i32,
        user: String,
        key_pem: String,
    },
    Local,
}

async fn deploy_shell_exec(target: &DeployExec, cmd: &str) -> Result<String, String> {
    match target {
        DeployExec::Ssh {
            host,
            port,
            user,
            key_pem,
        } => run_ssh_exec(host.clone(), *port, user.clone(), key_pem.clone(), cmd).await,
        DeployExec::Local => run_local_sh(cmd).await,
    }
}

fn build_deploy_exec_from_destination(
    destination_id: Option<Uuid>,
    dest_kind: &str,
    host_o: Option<String>,
    port_o: Option<i32>,
    user_o: Option<String>,
    cipher_o: Option<Vec<u8>>,
    cfg: &DeployWorkerConfig,
) -> Result<DeployExec, String> {
    if destination_id.is_none() {
        return Err("No destination configured for this application.".into());
    }
    if dest_kind == "docker_platform" {
        if !cfg.platform_docker_enabled {
            return Err("Platform Docker is disabled (DEPLOYWERK_PLATFORM_DOCKER_ENABLED).".into());
        }
        return Ok(DeployExec::Local);
    }
    let (host, ssh_port, ssh_user, ciphertext) = match (host_o, port_o, user_o, cipher_o) {
        (Some(h), Some(p), Some(u), Some(c)) if !h.is_empty() => (h, p, u, c),
        _ => return Err("Server not found for destination.".into()),
    };
    let key_pem = match decrypt_private_key(&cfg.server_key_encryption_key, &ciphertext) {
        Ok(b) => match String::from_utf8(b) {
            Ok(s) => s,
            Err(_) => return Err("Invalid stored SSH key.".into()),
        },
        Err(_) => return Err("Could not decrypt SSH key.".into()),
    };
    Ok(DeployExec::Ssh {
        host,
        port: ssh_port,
        user: ssh_user,
        key_pem,
    })
}

/// Returns `(full_command, redacted_command_for_logs)`.
fn docker_run_cmd(
    cname: &str,
    image: &str,
    cfg: &DeployWorkerConfig,
    traefik_hostname: Option<&str>,
    env_vars: &[(String, String, bool)],
    runtime_volumes: &[RuntimeVolumeMount],
    volumes_root_host_prefix: &str,
    isolated_network: Option<&str>,
) -> (String, String) {
    let mut s = format!(
        "docker run -d --name {} --restart unless-stopped",
        shell_single_quote(cname)
    );
    let mut s_log = s.clone();
    if let Some(iso) = isolated_network.map(str::trim).filter(|n| !n.is_empty()) {
        let frag = format!(" --network {}", shell_single_quote(iso));
        s.push_str(&frag);
        s_log.push_str(&frag);
    }
    if cfg.edge_mode == "traefik" {
        if let Some(host) = traefik_hostname.map(str::trim).filter(|h| !h.is_empty()) {
            let net = shell_single_quote(&cfg.traefik_docker_network);
            let safe_id: String = cname
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            let sid = if safe_id.is_empty() {
                "app".to_string()
            } else {
                safe_id
            };
            let rule = format!("Host(`{host}`)");
            let frag = format!(
                " --network {} --label {} --label {} --label {} --label {} --label {}",
                net,
                shell_single_quote("traefik.enable=true"),
                shell_single_quote(&format!("traefik.http.routers.dw-{sid}.rule={rule}")),
                shell_single_quote(&format!(
                    "traefik.http.routers.dw-{sid}.entrypoints=web"
                )),
                shell_single_quote(&format!(
                    "traefik.http.services.dw-{sid}.loadbalancer.server.port={}",
                    cfg.app_container_port
                )),
                shell_single_quote(&format!("traefik.http.routers.dw-{sid}.service=dw-{sid}"))
            );
            s.push_str(&frag);
            s_log.push_str(&frag);
        }
    }
    for v in runtime_volumes {
        let name = v.name.trim();
        let container_path = v.container_path.trim();
        if name.is_empty() || container_path.is_empty() || !container_path.starts_with('/') {
            continue;
        }
        let host_path = format!("{}/{}", volumes_root_host_prefix.trim_end_matches('/'), name);
        let frag = format!(
            " -v {}:{}",
            shell_single_quote(&host_path),
            shell_single_quote(container_path)
        );
        s.push_str(&frag);
        s_log.push_str(&frag);
    }
    for (k, v, is_secret) in env_vars {
        if !docker_env_name_valid(k) {
            continue;
        }
        s.push_str(&format!(" -e {}={}", k, shell_single_quote(v)));
        let vlog = if *is_secret {
            "[redacted]".to_string()
        } else {
            v.clone()
        };
        s_log.push_str(&format!(" -e {}={}", k, shell_single_quote(&vlog)));
    }
    s.push(' ');
    s.push_str(&shell_single_quote(image));
    s_log.push(' ');
    s_log.push_str(&shell_single_quote(image));
    (s, s_log)
}

async fn append_job_log(pool: &sqlx::PgPool, job_id: Uuid, line: &str) {
    let _ = sqlx::query(
        "UPDATE deploy_jobs SET log = log || $1 WHERE id = $2",
    )
    .bind(line)
    .bind(job_id)
    .execute(pool)
    .await;
}

async fn finish_job(
    pool: &sqlx::PgPool,
    job_id: Uuid,
    status: DeployJobStatus,
) -> Result<(), ApiError> {
    let now = Utc::now();
    sqlx::query(
        "UPDATE deploy_jobs SET status = $1, finished_at = $2 WHERE id = $3",
    )
    .bind(status.as_str())
    .bind(now)
    .bind(job_id)
    .execute(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    Ok(())
}

#[derive(FromRow)]
struct StorageBackendRow {
    endpoint_url: String,
    bucket: String,
    region: String,
    path_style: bool,
    access_key_ciphertext: Vec<u8>,
    secret_key_ciphertext: Vec<u8>,
}

async fn load_team_storage_backend(
    pool: &sqlx::PgPool,
    team_id: Uuid,
) -> Result<Option<StorageBackendRow>, ApiError> {
    let row: Option<StorageBackendRow> = sqlx::query_as(
        r#"SELECT endpoint_url, bucket, region, path_style,
                  access_key_ciphertext, secret_key_ciphertext
           FROM storage_backends
           WHERE team_id = $1
           ORDER BY created_at DESC
           LIMIT 1"#,
    )
    .bind(team_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    Ok(row)
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

fn hmac_sha256(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("hmac can take key of any size");
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}

fn uri_encode_path(raw: &str) -> String {
    // Encode per AWS SigV4 URI rules (RFC3986). Keep `/` as a path separator.
    let mut out = String::with_capacity(raw.len());
    for b in raw.as_bytes() {
        match *b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~'
            | b'/' => out.push(*b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

async fn s3_put_object_sigv4(
    endpoint_url: &str,
    bucket: &str,
    region: &str,
    path_style: bool,
    access_key: &str,
    secret_key: &str,
    key: &str,
    content_type: &str,
    body: Vec<u8>,
) -> Result<(), ApiError> {
    let endpoint_url = endpoint_url.trim();
    let bucket = bucket.trim();
    let key = key.trim_start_matches('/');
    let region = if region.trim().is_empty() {
        "us-east-1"
    } else {
        region.trim()
    };

    let base = reqwest::Url::parse(endpoint_url).map_err(|_| ApiError::Internal)?;
    let host = base.host_str().ok_or(ApiError::Internal)?.to_string();
    let scheme = base.scheme();
    let port = base.port();

    let canonical_uri = if path_style {
        format!("/{}/{}", uri_encode_path(bucket), uri_encode_path(key))
    } else {
        format!("/{}", uri_encode_path(key))
    };

    let mut url = if path_style {
        let mut u = base.clone();
        u.set_path(&format!("/{bucket}/{key}"));
        u
    } else {
        let mut u = base.clone();
        u.set_host(Some(&format!("{bucket}.{host}")))
            .map_err(|_| ApiError::Internal)?;
        u.set_path(&format!("/{key}"));
        u
    };
    if let Some(p) = port {
        url.set_port(Some(p)).ok();
    }

    let now = Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = now.format("%Y%m%d").to_string();

    let payload_hash = hex_sha256(&body);

    // Canonical headers must be lowercase and sorted.
    let host_header = match (scheme, port) {
        ("http", Some(80)) | ("https", Some(443)) | (_, None) => host.clone(),
        (_, Some(p)) => format!("{host}:{p}"),
    };

    let canonical_headers = format!(
        "host:{host_header}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\n"
    );
    let signed_headers = "host;x-amz-content-sha256;x-amz-date";

    let canonical_request = format!(
        "PUT\n{canonical_uri}\n\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
    );
    let canonical_request_hash = hex_sha256(canonical_request.as_bytes());

    let scope = format!("{date_stamp}/{region}/s3/aws4_request");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{canonical_request_hash}"
    );

    let k_date = hmac_sha256(format!("AWS4{secret_key}").as_bytes(), date_stamp.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, b"s3");
    let k_signing = hmac_sha256(&k_service, b"aws4_request");
    let signature = hex::encode(hmac_sha256(&k_signing, string_to_sign.as_bytes()));

    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{scope}, SignedHeaders={signed_headers}, Signature={signature}"
    );

    let client = reqwest::Client::new();
    client
        .put(url)
        .header("Host", host_header)
        .header("x-amz-date", amz_date)
        .header("x-amz-content-sha256", payload_hash)
        .header("Authorization", authorization)
        .header("Content-Type", content_type)
        .body(body)
        .send()
        .await
        .map_err(|_| ApiError::Internal)?
        .error_for_status()
        .map_err(|_| ApiError::Internal)?;

    Ok(())
}

async fn upload_deploy_job_artifacts(
    pool: &sqlx::PgPool,
    server_key_encryption_key: &[u8; 32],
    job_id: Uuid,
    team_id: Uuid,
) -> Result<(), ApiError> {
    let Some(sb) = load_team_storage_backend(pool, team_id).await? else {
        return Ok(());
    };

    let access_key = decrypt_private_key(server_key_encryption_key, &sb.access_key_ciphertext)
        .map_err(|_| ApiError::Internal)?;
    let secret_key = decrypt_private_key(server_key_encryption_key, &sb.secret_key_ciphertext)
        .map_err(|_| ApiError::Internal)?;
    let access_key = String::from_utf8(access_key).map_err(|_| ApiError::Internal)?;
    let secret_key = String::from_utf8(secret_key).map_err(|_| ApiError::Internal)?;

    let row: Option<(
        String,
        String,
        Option<String>,
        Option<String>,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
    )> = sqlx::query_as(
        r#"SELECT log, status, git_ref, git_sha, started_at, finished_at
           FROM deploy_jobs WHERE id = $1"#,
    )
    .bind(job_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    let Some((log, status, git_ref, git_sha, started_at, finished_at)) = row else {
        return Ok(());
    };

    let prefix = format!("teams/{team_id}/deploy-jobs/{job_id}");
    let log_key = format!("{prefix}/log.txt");
    let manifest_key = format!("{prefix}/manifest.json");

    s3_put_object_sigv4(
        &sb.endpoint_url,
        &sb.bucket,
        &sb.region,
        sb.path_style,
        &access_key,
        &secret_key,
        &log_key,
        "text/plain; charset=utf-8",
        log.into_bytes(),
    )
    .await?;

    let manifest = serde_json::json!({
        "team_id": team_id,
        "job_id": job_id,
        "status": status,
        "git_ref": git_ref,
        "git_sha": git_sha,
        "started_at": started_at,
        "finished_at": finished_at,
    });
    s3_put_object_sigv4(
        &sb.endpoint_url,
        &sb.bucket,
        &sb.region,
        sb.path_style,
        &access_key,
        &secret_key,
        &manifest_key,
        "application/json",
        serde_json::to_vec(&manifest).unwrap_or_default(),
    )
    .await?;

    sqlx::query(
        "UPDATE deploy_jobs SET log_object_key = $1, artifact_manifest_key = $2 WHERE id = $3",
    )
    .bind(&log_key)
    .bind(&manifest_key)
    .bind(job_id)
    .execute(pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(())
}

async fn finish_job_with_deploy_notify(
    pool: &sqlx::PgPool,
    server_key_encryption_key: &[u8; 32],
    job_id: Uuid,
    status: DeployJobStatus,
    team_id: Uuid,
    application_id: Uuid,
    application_name: &str,
    application_slug: &str,
    smtp: Option<crate::mail::SmtpSettings>,
) {
    let _ = upload_deploy_job_artifacts(pool, server_key_encryption_key, job_id, team_id).await;
    let _ = finish_job(pool, job_id, status).await;
    let event = match status {
        DeployJobStatus::Succeeded => "deploy_succeeded",
        DeployJobStatus::Failed => "deploy_failed",
        _ => return,
    };
    spawn_deploy_notifications(
        pool.clone(),
        team_id,
        job_id,
        application_id,
        application_name,
        application_slug,
        event,
        Some(status.as_str().to_string()),
        smtp,
    );
}

/// POST JSON `{ phase, job_id, application_id, application_slug, docker_image }` with a 120s timeout.
async fn invoke_deploy_hook_http(
    phase: &str,
    url: Option<&str>,
    job_id: Uuid,
    application_id: Uuid,
    slug: &str,
    docker_image: &str,
) -> Result<(), String> {
    let Some(u) = url.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    if !u.starts_with("http://") && !u.starts_with("https://") {
        return Err(format!("hook URL must be http(s): {u}"));
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let body = serde_json::json!({
        "phase": phase,
        "job_id": job_id,
        "application_id": application_id,
        "application_slug": slug,
        "docker_image": docker_image,
    });
    let resp = client
        .post(u)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} from hook", resp.status()));
    }
    Ok(())
}

#[derive(FromRow)]
struct DeployJobApplicationRow {
    job_kind: String,
    pr_number: Option<i32>,
    docker_image: String,
    slug: String,
    destination_id: Option<Uuid>,
    build_image_from_git: bool,
    git_repo_url: Option<String>,
    git_build_ref: String,
    dockerfile_path: String,
    auto_hostname: Option<String>,
    runtime_volumes_json: serde_json::Value,
    dest_kind: String,
    host: Option<String>,
    ssh_port: Option<i32>,
    ssh_user: Option<String>,
    ssh_private_key_ciphertext: Option<Vec<u8>>,
    team_id: Uuid,
    name: String,
    previous_deployed_image: Option<String>,
    deploy_strategy: String,
    pre_deploy_hook_url: Option<String>,
    post_deploy_hook_url: Option<String>,
}

/// Runs a single deploy job (SSH/local Docker). Safe to call from `deploywerk-deploy-worker` after claiming a row.
pub async fn execute_deploy_job(
    pool: sqlx::PgPool,
    cfg: DeployWorkerConfig,
    job_id: Uuid,
    application_id: Uuid,
) {
    let now = Utc::now();
    sqlx::query(
        "UPDATE deploy_jobs SET status = 'running', started_at = $1 WHERE id = $2",
    )
    .bind(now)
    .bind(job_id)
    .execute(&pool)
    .await
    .ok();

        let row: Option<DeployJobApplicationRow> = sqlx::query_as(
            r#"SELECT COALESCE(j.job_kind, 'standard') AS job_kind, j.pr_number,
                      a.docker_image, a.slug, a.destination_id,
                      a.build_image_from_git, a.git_repo_url, a.git_build_ref, a.dockerfile_path,
                      a.auto_hostname,
                      COALESCE(a.runtime_volumes_json, '[]'::jsonb) AS runtime_volumes_json,
                      COALESCE(d.kind, '') AS dest_kind,
                      s.host, s.ssh_port, s.ssh_user, s.ssh_private_key_ciphertext,
                      p.team_id, a.name,
                      a.previous_deployed_image,
                      COALESCE(NULLIF(trim(j.deploy_strategy), ''), a.deploy_strategy, 'standard') AS deploy_strategy,
                      NULLIF(trim(a.pre_deploy_hook_url), '') AS pre_deploy_hook_url,
                      NULLIF(trim(a.post_deploy_hook_url), '') AS post_deploy_hook_url
               FROM deploy_jobs j
               JOIN applications a ON a.id = j.application_id
               JOIN environments e ON e.id = a.environment_id
               JOIN projects p ON p.id = e.project_id
               LEFT JOIN destinations d ON d.id = a.destination_id
               LEFT JOIN servers s ON s.id = d.server_id
               WHERE j.id = $1 AND a.id = $2"#,
        )
        .bind(job_id)
        .bind(application_id)
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten();

        let Some(DeployJobApplicationRow {
            job_kind,
            pr_number,
            docker_image,
            slug,
            destination_id,
            build_image_from_git,
            git_repo_url,
            git_build_ref,
            dockerfile_path,
            auto_hostname,
            runtime_volumes_json,
            dest_kind,
            host: host_o,
            ssh_port: port_o,
            ssh_user: user_o,
            ssh_private_key_ciphertext: cipher_o,
            team_id,
            name: app_name,
            previous_deployed_image,
            deploy_strategy,
            pre_deploy_hook_url,
            post_deploy_hook_url,
        }) = row
        else {
            append_job_log(&pool, job_id, "Deploy job or application not found.\n").await;
            let _ = finish_job(&pool, job_id, DeployJobStatus::Failed).await;
            return;
        };

        let runtime_volumes: Vec<RuntimeVolumeMount> =
            serde_json::from_value(runtime_volumes_json).unwrap_or_default();

        let job_kind_s = job_kind.trim().to_string();
        let is_preview_destroy = job_kind_s == "pr_preview_destroy";
        let strat = deploy_strategy.trim();
        if !is_preview_destroy {
            append_job_log(
                &pool,
                job_id,
                &format!("Deploy strategy: {strat} (worker uses standard container replace; advanced routing is staged separately).\n"),
            )
            .await;
        }

        if !is_preview_destroy {
            spawn_deploy_notifications(
                pool.clone(),
                team_id,
                job_id,
                application_id,
                &app_name,
                &slug,
                "deploy_started",
                Some("running".to_string()),
                cfg.smtp_settings.clone(),
            );
        }

        let exec_target = match build_deploy_exec_from_destination(
            destination_id,
            &dest_kind,
            host_o,
            port_o,
            user_o,
            cipher_o,
            &cfg,
        ) {
            Ok(e) => e,
            Err(msg) => {
                append_job_log(&pool, job_id, &format!("{msg}\n")).await;
                finish_job_with_deploy_notify(
                    &pool,
                    &cfg.server_key_encryption_key,
                    job_id,
                    DeployJobStatus::Failed,
                    team_id,
                    application_id,
                    &app_name,
                    &slug,
                    cfg.smtp_settings.clone(),
                )
                .await;
                return;
            }
        };

        // Preflight: ensure Docker is reachable on the execution target.
        // This is especially important in single-server mode where git builds rely on local Docker.
        if !is_preview_destroy {
            let info_cmd = "docker info";
            match deploy_shell_exec(&exec_target, info_cmd).await {
                Ok(_) => {}
                Err(e) => {
                    append_job_log(&pool, job_id, &format!("Preflight failed: {info_cmd}: {e}\n")).await;
                    finish_job_with_deploy_notify(
                        &pool,
                        &cfg.server_key_encryption_key,
                        job_id,
                        DeployJobStatus::Failed,
                        team_id,
                        application_id,
                        &app_name,
                        &slug,
                        cfg.smtp_settings.clone(),
                    )
                    .await;
                    return;
                }
            }
            if cfg.edge_mode == "traefik" {
                let net = cfg.traefik_docker_network.trim();
                if !net.is_empty() {
                    let net_cmd = format!("docker network inspect {} >/dev/null 2>&1", shell_single_quote(net));
                    if let Err(e) = deploy_shell_exec(&exec_target, &net_cmd).await {
                        append_job_log(
                            &pool,
                            job_id,
                            &format!("Preflight note: traefik network missing/unreachable ({net}): {e}\n"),
                        )
                        .await;
                    }
                }
            }
        }

        if is_preview_destroy {
            let Some(pr) = pr_number else {
                append_job_log(&pool, job_id, "pr_preview_destroy job missing pr_number.\n").await;
                let _ = finish_job(&pool, job_id, DeployJobStatus::Failed).await;
                return;
            };
            let cname = preview_docker_container_name(&slug, application_id, pr);
            append_job_log(
                &pool,
                job_id,
                &format!("PR preview teardown: removing {}\n", cname),
            )
            .await;
            let rm_cmd = format!("docker rm -f {}", shell_single_quote(&cname));
            match deploy_shell_exec(&exec_target, &rm_cmd).await {
                Ok(out) => {
                    append_job_log(&pool, job_id, &out).await;
                    append_job_log(&pool, job_id, "\nPreview removed.\n").await;
                    let _ = finish_job(&pool, job_id, DeployJobStatus::Succeeded).await;
                }
                Err(e) => {
                    append_job_log(
                        &pool,
                        job_id,
                        &format!("preview rm note (container may already be gone): {e}\n"),
                    )
                    .await;
                    let _ = finish_job(&pool, job_id, DeployJobStatus::Succeeded).await;
                }
            }
            if cfg.pr_preview_isolated_network {
                let net = preview_docker_network_name(&slug, application_id, pr);
                let nr = format!("docker network rm {}", shell_single_quote(&net));
                match deploy_shell_exec(&exec_target, &nr).await {
                    Ok(o) if !o.trim().is_empty() => {
                        append_job_log(&pool, job_id, &format!("{o}\n")).await;
                    }
                    Err(e) => {
                        append_job_log(
                            &pool,
                            job_id,
                            &format!("preview network rm note (may already be gone): {e}\n"),
                        )
                        .await;
                    }
                    _ => {}
                }
            }
            return;
        }

        let short = application_id.as_simple().to_string();
        let short8 = &short[..short.len().min(8)];
        let slug_part = docker_name_part(&slug);
        let cname = if job_kind_s == "pr_preview" {
            let Some(pr) = pr_number else {
                append_job_log(&pool, job_id, "pr_preview job missing pr_number.\n").await;
                finish_job_with_deploy_notify(
                    &pool,
                    &cfg.server_key_encryption_key,
                    job_id,
                    DeployJobStatus::Failed,
                    team_id,
                    application_id,
                    &app_name,
                    &slug,
                    cfg.smtp_settings.clone(),
                )
                .await;
                return;
            };
            preview_docker_container_name(&slug, application_id, pr)
        } else if slug_part.is_empty() {
            format!("dw-{}", short8)
        } else {
            format!("dw-{}-{}", slug_part, short8)
        };

        let env_rows_raw: Vec<(String, String, bool)> = sqlx::query_as(
            "SELECT key, value, is_secret FROM application_env_vars WHERE application_id = $1 ORDER BY key",
        )
        .bind(application_id)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

        let env_rows = match crate::team_secrets::resolve_dw_secret_env_values(
            &pool,
            team_id,
            &cfg.server_key_encryption_key,
            env_rows_raw,
        )
        .await
        {
            Ok(r) => r,
            Err(msg) => {
                append_job_log(
                    &pool,
                    job_id,
                    &format!("secret resolution failed: {msg}\n"),
                )
                .await;
                finish_job_with_deploy_notify(
                    &pool,
                    &cfg.server_key_encryption_key,
                    job_id,
                    DeployJobStatus::Failed,
                    team_id,
                    application_id,
                    &app_name,
                    &slug,
                    cfg.smtp_settings.clone(),
                )
                .await;
                return;
            }
        };

        if let Err(msg) = validate_env_for_docker_run(&env_rows) {
            append_job_log(&pool, job_id, &format!("{msg}\n")).await;
            finish_job_with_deploy_notify(
                &pool,
                &cfg.server_key_encryption_key,
                job_id,
                DeployJobStatus::Failed,
                team_id,
                application_id,
                &app_name,
                &slug,
                cfg.smtp_settings.clone(),
            )
            .await;
            return;
        }

        if !is_preview_destroy {
            if let Err(msg) = invoke_deploy_hook_http(
                "pre_deploy",
                pre_deploy_hook_url.as_deref(),
                job_id,
                application_id,
                &slug,
                docker_image.trim(),
            )
            .await
            {
                append_job_log(&pool, job_id, &format!("pre_deploy hook failed: {msg}\n")).await;
                finish_job_with_deploy_notify(
                    &pool,
                    &cfg.server_key_encryption_key,
                    job_id,
                    DeployJobStatus::Failed,
                    team_id,
                    application_id,
                    &app_name,
                    &slug,
                    cfg.smtp_settings.clone(),
                )
                .await;
                return;
            }
        }

        let build_from_remote_git = build_image_from_git
            && git_repo_url
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);

        let image_for_run: String = if job_kind_s == "rollback" {
            let prev = previous_deployed_image
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty());
            let Some(p) = prev else {
                append_job_log(
                    &pool,
                    job_id,
                    "No previous deployment image to roll back to.\n",
                )
                .await;
                finish_job_with_deploy_notify(
                    &pool,
                    &cfg.server_key_encryption_key,
                    job_id,
                    DeployJobStatus::Failed,
                    team_id,
                    application_id,
                    &app_name,
                    &slug,
                    cfg.smtp_settings.clone(),
                )
                .await;
                return;
            };
            append_job_log(
                &pool,
                job_id,
                &format!("Rollback: pulling {} (continues if image is local-only)\n", p),
            )
            .await;
            let pull_cmd = format!("docker pull {}", shell_single_quote(p));
            append_job_log(&pool, job_id, &format!("Running: {pull_cmd}\n")).await;
            match deploy_shell_exec(&exec_target, &pull_cmd).await {
                Ok(out) => {
                    append_job_log(&pool, job_id, &out).await;
                    append_job_log(&pool, job_id, "\n").await;
                }
                Err(e) => {
                    append_job_log(
                        &pool,
                        job_id,
                        &format!("docker pull note (using local image if present): {e}\n"),
                    )
                    .await;
                }
            }
            p.to_string()
        } else if build_from_remote_git {
            if dest_kind.trim() != "docker_platform" {
                append_job_log(
                    &pool,
                    job_id,
                    "Git builds are only supported on docker_platform destinations in single-server mode.\n",
                )
                .await;
                finish_job_with_deploy_notify(
                    &pool,
                    &cfg.server_key_encryption_key,
                    job_id,
                    DeployJobStatus::Failed,
                    team_id,
                    application_id,
                    &app_name,
                    &slug,
                    cfg.smtp_settings.clone(),
                )
                .await;
                return;
            }
            let url = git_repo_url.as_deref().map(str::trim).unwrap_or("");
            let br = git_build_ref.trim();
            if br.is_empty() {
                append_job_log(&pool, job_id, "git_build_ref is empty; cannot clone.\n").await;
                finish_job_with_deploy_notify(
                    &pool,
                    &cfg.server_key_encryption_key,
                    job_id,
                    DeployJobStatus::Failed,
                    team_id,
                    application_id,
                    &app_name,
                    &slug,
                    cfg.smtp_settings.clone(),
                )
                .await;
                return;
            }
            let df = dockerfile_path.trim();
            let df = if df.is_empty() { "Dockerfile" } else { df };
            let work = format!("/tmp/dw-build-{}", short8);
            let img_tag = format!("dwbuild-{}:{}", slug_part, short8);

            let clean_cmd = format!("rm -rf {}", shell_single_quote(&work));
            append_job_log(&pool, job_id, "Git build: preparing workdir\n").await;
            let _ = deploy_shell_exec(&exec_target, &clean_cmd).await;

            let cache_root = cfg.git_cache_root.trim();
            let repo_norm = normalize_git_remote_path(url);
            let repo_dir = repo_norm
                .replace('\\', "/")
                .trim_matches('/')
                .replace('/', "__");
            let mirror = format!("{}/{}.git", cache_root.trim_end_matches('/'), repo_dir);

            let prep_cache_cmd = format!("mkdir -p {}", shell_single_quote(cache_root));
            let _ = deploy_shell_exec(&exec_target, &prep_cache_cmd).await;

            let mirror_cmd = format!(
                "if [ -d {m} ]; then git -C {m} remote update --prune; else git clone --mirror {u} {m}; fi",
                m = shell_single_quote(&mirror),
                u = shell_single_quote(url),
            );
            append_job_log(&pool, job_id, &format!("Running: {mirror_cmd}\n")).await;
            match deploy_shell_exec(&exec_target, &mirror_cmd).await {
                Ok(out) => {
                    append_job_log(&pool, job_id, &out).await;
                    append_job_log(&pool, job_id, "\n").await;
                }
                Err(e) => {
                    append_job_log(&pool, job_id, &format!("git cache update failed: {e}\n")).await;
                    finish_job_with_deploy_notify(
                        &pool,
                        &cfg.server_key_encryption_key,
                        job_id,
                        DeployJobStatus::Failed,
                        team_id,
                        application_id,
                        &app_name,
                        &slug,
                        cfg.smtp_settings.clone(),
                    )
                    .await;
                    return;
                }
            }

            let clone_cmd = format!(
                "git clone {m} {w} && git -C {w} checkout {b}",
                m = shell_single_quote(&mirror),
                w = shell_single_quote(&work),
                b = shell_single_quote(br),
            );
            append_job_log(&pool, job_id, &format!("Running: {clone_cmd}\n")).await;
            match deploy_shell_exec(&exec_target, &clone_cmd).await {
                Ok(out) => {
                    append_job_log(&pool, job_id, &out).await;
                    append_job_log(&pool, job_id, "\n").await;
                }
                Err(e) => {
                    append_job_log(&pool, job_id, &format!("git checkout failed: {e}\n")).await;
                    finish_job_with_deploy_notify(
                        &pool,
                        &cfg.server_key_encryption_key,
                        job_id,
                        DeployJobStatus::Failed,
                        team_id,
                        application_id,
                        &app_name,
                        &slug,
                        cfg.smtp_settings.clone(),
                    )
                    .await;
                    return;
                }
            }

            let build_cmd = format!(
                "docker build -t {} -f {} {}",
                shell_single_quote(&img_tag),
                shell_single_quote(df),
                shell_single_quote(&work)
            );
            append_job_log(&pool, job_id, &format!("Running: {build_cmd}\n")).await;
            match deploy_shell_exec(&exec_target, &build_cmd).await
            {
                Ok(out) => {
                    append_job_log(&pool, job_id, &out).await;
                    append_job_log(&pool, job_id, "\n").await;
                }
                Err(e) => {
                    append_job_log(&pool, job_id, &format!("docker build failed: {e}\n")).await;
                    finish_job_with_deploy_notify(
                        &pool,
                        &cfg.server_key_encryption_key,
                        job_id,
                        DeployJobStatus::Failed,
                        team_id,
                        application_id,
                        &app_name,
                        &slug,
                        cfg.smtp_settings.clone(),
                    )
                    .await;
                    return;
                }
            }
            img_tag
        } else {
            let pull_cmd = format!("docker pull {}", shell_single_quote(&docker_image));
            append_job_log(&pool, job_id, &format!("Running: {pull_cmd}\n")).await;
            match deploy_shell_exec(&exec_target, &pull_cmd).await
            {
                Ok(out) => {
                    append_job_log(&pool, job_id, &out).await;
                    append_job_log(&pool, job_id, "\n").await;
                }
                Err(e) => {
                    append_job_log(&pool, job_id, &format!("pull failed: {e}\n")).await;
                    finish_job_with_deploy_notify(
                        &pool,
                        &cfg.server_key_encryption_key,
                        job_id,
                        DeployJobStatus::Failed,
                        team_id,
                        application_id,
                        &app_name,
                        &slug,
                        cfg.smtp_settings.clone(),
                    )
                    .await;
                    return;
                }
            }
            docker_image
        };

        let rm_cmd = format!("docker rm -f {}", shell_single_quote(&cname));
        let _ = deploy_shell_exec(&exec_target, &rm_cmd).await;

        let isolated_network: Option<String> =
            if job_kind_s == "pr_preview" && cfg.pr_preview_isolated_network {
                if let Some(pr) = pr_number {
                    let net = preview_docker_network_name(&slug, application_id, pr);
                    let create = format!("docker network create {}", shell_single_quote(&net));
                    append_job_log(
                        &pool,
                        job_id,
                        &format!("PR preview isolated network: {create}\n"),
                    )
                    .await;
                    match deploy_shell_exec(&exec_target, &create).await {
                        Ok(out) => {
                            append_job_log(&pool, job_id, &out).await;
                        }
                        Err(e) => {
                            append_job_log(
                                &pool,
                                job_id,
                                &format!("network create note (may already exist): {e}\n"),
                            )
                            .await;
                        }
                    }
                    Some(net)
                } else {
                    None
                }
            } else {
                None
            };

        let traefik_hostname: Option<String> = if job_kind_s == "pr_preview" {
            match (pr_number, cfg.apps_base_domain.as_ref()) {
                (Some(pr), Some(base)) => Some(preview_traefik_hostname(base, pr, &slug)),
                _ => None,
            }
        } else {
            auto_hostname
                .as_ref()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };

        if cfg.edge_mode == "traefik"
            && traefik_hostname.as_ref().map(|s| s.as_str()).filter(|s| !s.is_empty()).is_none()
        {
            let msg = if job_kind_s == "pr_preview" {
                "Note: DEPLOYWERK_EDGE_MODE=traefik but no preview hostname (set DEPLOYWERK_APPS_BASE_DOMAIN for pr-* hostnames) or no auto_hostname for standard deploys.\n"
            } else {
                "Note: DEPLOYWERK_EDGE_MODE=traefik but no auto_hostname; add Traefik labels manually or set DEPLOYWERK_APPS_BASE_DOMAIN.\n"
            };
            append_job_log(&pool, job_id, msg).await;
        }

        let volumes_prefix = format!(
            "{}/teams/{}/applications/{}",
            cfg.volumes_root.trim_end_matches('/'),
            team_id,
            application_id
        );
        if !runtime_volumes.is_empty() {
            for v in &runtime_volumes {
                let name = v.name.trim();
                let container_path = v.container_path.trim();
                if name.is_empty() || container_path.is_empty() || !container_path.starts_with('/') {
                    continue;
                }
                let mk = format!(
                    "mkdir -p {}",
                    shell_single_quote(&format!("{}/{}", volumes_prefix, name))
                );
                let _ = deploy_shell_exec(&exec_target, &mk).await;
            }
        }

        let (run_cmd, run_cmd_log) = docker_run_cmd(
            &cname,
            &image_for_run,
            &cfg,
            traefik_hostname.as_deref(),
            &env_rows,
            &runtime_volumes,
            &volumes_prefix,
            isolated_network.as_deref(),
        );
        append_job_log(&pool, job_id, &format!("Running: {run_cmd_log}\n")).await;

        match deploy_shell_exec(&exec_target, &run_cmd).await {
            Ok(out) => {
                append_job_log(&pool, job_id, &out).await;
                append_job_log(&pool, job_id, "\nDeploy finished.\n").await;
                if job_kind_s == "standard" || job_kind_s == "rollback" {
                    let res = if job_kind_s == "standard" {
                        sqlx::query(
                            r#"UPDATE applications SET previous_deployed_image = last_deployed_image,
                                   last_deployed_image = $1 WHERE id = $2"#,
                        )
                        .bind(&image_for_run)
                        .bind(application_id)
                        .execute(&pool)
                        .await
                    } else {
                        sqlx::query(
                            r#"UPDATE applications SET previous_deployed_image = last_deployed_image,
                                   last_deployed_image = previous_deployed_image WHERE id = $1"#,
                        )
                        .bind(application_id)
                        .execute(&pool)
                        .await
                    };
                    if res.is_err() {
                        tracing::warn!(%application_id, "failed to persist deploy image history");
                    }
                }
                if let Err(msg) = invoke_deploy_hook_http(
                    "post_deploy",
                    post_deploy_hook_url.as_deref(),
                    job_id,
                    application_id,
                    &slug,
                    image_for_run.trim(),
                )
                .await
                {
                    append_job_log(&pool, job_id, &format!("post_deploy hook failed: {msg}\n")).await;
                    finish_job_with_deploy_notify(
                        &pool,
                        &cfg.server_key_encryption_key,
                        job_id,
                        DeployJobStatus::Failed,
                        team_id,
                        application_id,
                        &app_name,
                        &slug,
                        cfg.smtp_settings.clone(),
                    )
                    .await;
                    return;
                }
                finish_job_with_deploy_notify(
                    &pool,
                    &cfg.server_key_encryption_key,
                    job_id,
                    DeployJobStatus::Succeeded,
                    team_id,
                    application_id,
                    &app_name,
                    &slug,
                    cfg.smtp_settings.clone(),
                )
                .await;
            }
            Err(e) => {
                append_job_log(&pool, job_id, &format!("run failed: {e}\n")).await;
                finish_job_with_deploy_notify(
                    &pool,
                    &cfg.server_key_encryption_key,
                    job_id,
                    DeployJobStatus::Failed,
                    team_id,
                    application_id,
                    &app_name,
                    &slug,
                    cfg.smtp_settings.clone(),
                )
                .await;
            }
        }
}

/// Claim one `queued` job using `FOR UPDATE SKIP LOCKED` (Postgres). Returns `None` if the queue is empty.
pub async fn try_claim_next_queued_deploy_job(
    pool: &sqlx::PgPool,
) -> anyhow::Result<Option<(Uuid, Uuid)>> {
    let mut tx = pool.begin().await?;
    let row: Option<(Uuid, Uuid)> = sqlx::query_as(
        r#"SELECT id, application_id FROM deploy_jobs
           WHERE status = 'queued'
           ORDER BY created_at ASC
           FOR UPDATE SKIP LOCKED
           LIMIT 1"#,
    )
    .fetch_optional(&mut *tx)
    .await?;

    let Some((job_id, application_id)) = row else {
        tx.commit().await?;
        return Ok(None);
    };

    let now = Utc::now();
    let updated = sqlx::query(
        "UPDATE deploy_jobs SET status = 'running', started_at = $1 WHERE id = $2 AND status = 'queued'",
    )
    .bind(now)
    .bind(job_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if updated == 0 {
        tx.commit().await?;
        return Ok(None);
    }

    tx.commit().await?;
    Ok(Some((job_id, application_id)))
}

fn run_deploy_worker(
    pool: sqlx::PgPool,
    cfg: DeployWorkerConfig,
    job_id: Uuid,
    application_id: Uuid,
) {
    tokio::spawn(execute_deploy_job(pool, cfg, job_id, application_id));
}

fn shell_single_quote(s: &str) -> String {
    let escaped = s.replace('\'', "'\"'\"'");
    format!("'{escaped}'")
}

struct DeployPreflight {
    deploy_locked: bool,
    deploy_lock_reason: Option<String>,
    deploy_schedule_json: Option<String>,
    require_deploy_approval: bool,
    deploy_strategy: String,
}

async fn load_deploy_preflight(
    pool: &sqlx::PgPool,
    application_id: Uuid,
) -> Result<DeployPreflight, ApiError> {
    let row: Option<(bool, Option<String>, Option<String>, bool, String)> = sqlx::query_as(
        r#"SELECT e.deploy_locked, e.deploy_lock_reason, e.deploy_schedule_json,
                  a.require_deploy_approval, a.deploy_strategy
           FROM applications a
           JOIN environments e ON e.id = a.environment_id
           WHERE a.id = $1"#,
    )
    .bind(application_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    let Some((
        deploy_locked,
        deploy_lock_reason,
        deploy_schedule_json,
        require_deploy_approval,
        deploy_strategy,
    )) = row
    else {
        return Err(ApiError::NotFound);
    };
    Ok(DeployPreflight {
        deploy_locked,
        deploy_lock_reason,
        deploy_schedule_json,
        require_deploy_approval,
        deploy_strategy,
    })
}

fn dispatch_deploy_worker_if_needed(
    state: &Arc<AppState>,
    job_id: Uuid,
    application_id: Uuid,
) {
    let pool = state.pool.clone();
    let cfg = state.deploy_worker.clone();
    if state.deploy_dispatch_inline {
        run_deploy_worker(pool, cfg, job_id, application_id);
    } else {
        tracing::info!(%job_id, %application_id, "deploy queued for external worker");
    }
}

async fn enqueue_deploy_inner(
    state: Arc<AppState>,
    application_id: Uuid,
    git: Option<(String, String)>,
    git_base_sha: Option<String>,
    job_kind: &str,
    pr_number: Option<i32>,
    respect_deploy_approval: bool,
) -> Result<EnqueueDeployResponse, ApiError> {
    let pf = load_deploy_preflight(&state.pool, application_id).await?;
    if pf.deploy_locked {
        let reason = pf
            .deploy_lock_reason
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("environment is locked");
        return Err(ApiError::BadRequest(format!("deployments blocked: {reason}")));
    }
    if !deploy_schedule_allows_now(pf.deploy_schedule_json.as_deref())
        .map_err(|e| ApiError::BadRequest(e.into()))?
    {
        return Err(ApiError::BadRequest(
            "outside configured deploy schedule for this environment".into(),
        ));
    }

    let job_kind_s = job_kind.trim();
    let needs_approval = respect_deploy_approval
        && pf.require_deploy_approval
        && (job_kind_s == "standard");
    let status_str = if needs_approval {
        "pending_approval"
    } else {
        "queued"
    };
    let initial_log = if needs_approval {
        "Waiting for a team admin or owner to approve this deploy.\n"
    } else {
        ""
    };

    let job_id = Uuid::new_v4();
    let now = Utc::now();
    let (git_ref, git_sha) = match &git {
        Some((r, s)) => (Some(r.as_str()), Some(s.as_str())),
        None => (None, None),
    };
    let base_store = git_base_sha
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    sqlx::query(
        r#"INSERT INTO deploy_jobs (id, application_id, status, log, created_at, started_at, finished_at, git_ref, git_sha, git_base_sha, job_kind, pr_number, deploy_strategy)
           VALUES ($1, $2, $3, $4, $5, NULL, NULL, $6, $7, $8, $9, $10, $11)"#,
    )
    .bind(job_id)
    .bind(application_id)
    .bind(status_str)
    .bind(initial_log)
    .bind(now)
    .bind(git_ref)
    .bind(git_sha)
    .bind(&base_store)
    .bind(job_kind)
    .bind(pr_number)
    .bind(&pf.deploy_strategy)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    if needs_approval {
        // Notify via the existing endpoint system (webhook/email), so teams get a signal that a deploy is queued.
        let ctx: Option<(Uuid, String, String)> = sqlx::query_as(
            r#"SELECT p.team_id, a.name, a.slug
               FROM applications a
               JOIN environments e ON e.id = a.environment_id
               JOIN projects p ON p.id = e.project_id
               WHERE a.id = $1"#,
        )
        .bind(application_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
        if let Some((team_id, app_name, app_slug)) = ctx {
            spawn_deploy_notifications(
                state.pool.clone(),
                team_id,
                job_id,
                application_id,
                &app_name,
                &app_slug,
                "deploy_pending_approval",
                Some("pending_approval".to_string()),
                state.smtp_settings.clone(),
            );
        }
    }

    if status_str == "queued" {
        dispatch_deploy_worker_if_needed(&state, job_id, application_id);
    }

    Ok(EnqueueDeployResponse {
        job_id,
        status: if needs_approval {
            DeployJobStatus::PendingApproval
        } else {
            DeployJobStatus::Queued
        },
    })
}

/// Enqueue deploy when `application_id` belongs to `team_id` (web CLI / internal callers).
pub async fn enqueue_deploy_application_in_team(
    state: Arc<AppState>,
    user_id: Uuid,
    team_id: Uuid,
    application_id: Uuid,
) -> Result<EnqueueDeployResponse, ApiError> {
    let tid: Option<Uuid> = sqlx::query_scalar(
        r#"SELECT p.team_id FROM applications a
           JOIN environments e ON e.id = a.environment_id
           JOIN projects p ON p.id = e.project_id
           WHERE a.id = $1"#,
    )
    .bind(application_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    let Some(tid) = tid else {
        return Err(ApiError::NotFound);
    };
    if tid != team_id {
        return Err(ApiError::Forbidden);
    }
    require_application_deploy(&state.pool, user_id, team_id, application_id).await?;
    enqueue_deploy_inner(
        state,
        application_id,
        None,
        None,
        "standard",
        None,
        true,
    )
    .await
}

/// `POST /api/v1/applications/{application_id}/deploy` — resolve team from application graph.
pub async fn deploy_global(
    State(state): State<Arc<AppState>>,
    Path(application_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<EnqueueDeployResponse>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_deploy()?;

    let team_id: Option<Uuid> = sqlx::query_scalar(
        r#"SELECT p.team_id FROM applications a
           JOIN environments e ON e.id = a.environment_id
           JOIN projects p ON p.id = e.project_id
           WHERE a.id = $1"#,
    )
    .bind(application_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some(tid) = team_id else {
        return Err(ApiError::NotFound);
    };

    require_application_deploy(&state.pool, p.user_id, tid, application_id).await?;

    let body =
        enqueue_deploy_inner(state.clone(), application_id, None, None, "standard", None, true).await?;
    let ip = crate::auth::peer_ip_from_headers(&headers).map(|i| i.to_string());
    try_log_team_audit(
        &state.pool,
        tid,
        p.user_id,
        "deploy_enqueued",
        "deploy_job",
        Some(body.job_id),
        serde_json::json!({ "application_id": application_id, "source": "api_global" }),
        ip,
    )
    .await;
    Ok((StatusCode::ACCEPTED, Json(body)))
}

async fn enqueue_deploy_scoped(
    State(state): State<Arc<AppState>>,
    Path((team_id, project_id, environment_id, application_id)): Path<(Uuid, Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<EnqueueDeployResponse>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_deploy()?;
    ensure_env_in_team_project(&state.pool, team_id, project_id, environment_id).await?;
    ensure_application_in_env(&state.pool, application_id, environment_id).await?;
    require_application_deploy(&state.pool, p.user_id, team_id, application_id).await?;

    let body =
        enqueue_deploy_inner(state.clone(), application_id, None, None, "standard", None, true).await?;
    let ip = crate::auth::peer_ip_from_headers(&headers).map(|i| i.to_string());
    try_log_team_audit(
        &state.pool,
        team_id,
        p.user_id,
        "deploy_enqueued",
        "deploy_job",
        Some(body.job_id),
        serde_json::json!({ "application_id": application_id, "source": "api_scoped" }),
        ip,
    )
    .await;
    Ok((StatusCode::ACCEPTED, Json(body)))
}

async fn enqueue_rollback_scoped(
    State(state): State<Arc<AppState>>,
    Path((team_id, project_id, environment_id, application_id)): Path<(Uuid, Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<EnqueueDeployResponse>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_deploy()?;
    ensure_env_in_team_project(&state.pool, team_id, project_id, environment_id).await?;
    ensure_application_in_env(&state.pool, application_id, environment_id).await?;
    require_application_deploy(&state.pool, p.user_id, team_id, application_id).await?;

    let row: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT previous_deployed_image FROM applications WHERE id = $1 AND environment_id = $2",
    )
    .bind(application_id)
    .bind(environment_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((prev_col,)) = row else {
        return Err(ApiError::NotFound);
    };
    let Some(ref s) = prev_col else {
        return Err(ApiError::BadRequest(
            "no previous deployment to roll back to".into(),
        ));
    };
    if s.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "no previous deployment to roll back to".into(),
        ));
    }

    let body =
        enqueue_deploy_inner(state, application_id, None, None, "rollback", None, false).await?;
    Ok((StatusCode::ACCEPTED, Json(body)))
}

async fn approve_deploy_job(
    State(state): State<Arc<AppState>>,
    Path((team_id, job_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<EnqueueDeployResponse>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    let row: Option<(Uuid, String)> = sqlx::query_as(
        r#"SELECT j.application_id, j.status FROM deploy_jobs j
           JOIN applications a ON a.id = j.application_id
           JOIN environments e ON e.id = a.environment_id
           JOIN projects p ON p.id = e.project_id
           WHERE j.id = $1 AND p.team_id = $2"#,
    )
    .bind(job_id)
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((application_id, status)) = row else {
        return Err(ApiError::NotFound);
    };
    require_application_mutate(&state.pool, p.user_id, team_id, application_id).await?;
    if status != "pending_approval" {
        return Err(ApiError::BadRequest(
            "job is not waiting for approval".into(),
        ));
    }

    let now = Utc::now();
    let n = sqlx::query(
        r#"UPDATE deploy_jobs SET status = 'queued', approved_at = $1, approved_by_user_id = $2
           WHERE id = $3 AND status = 'pending_approval'"#,
    )
    .bind(now)
    .bind(p.user_id)
    .bind(job_id)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?
    .rows_affected();

    if n == 0 {
        return Err(ApiError::Conflict("job already processed".into()));
    }

    dispatch_deploy_worker_if_needed(&state, job_id, application_id);

    try_log_team_audit(
        &state.pool,
        team_id,
        p.user_id,
        "deploy_job.approve",
        "deploy_job",
        Some(job_id),
        serde_json::json!({ "application_id": application_id }),
        None,
    )
    .await;

    Ok(Json(EnqueueDeployResponse {
        job_id,
        status: DeployJobStatus::Queued,
    }))
}

async fn list_deploy_jobs(
    State(state): State<Arc<AppState>>,
    Path((team_id, project_id, environment_id, application_id)): Path<(Uuid, Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<Vec<DeployJobSummary>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    ensure_env_in_team_project(&state.pool, team_id, project_id, environment_id).await?;
    ensure_application_in_env(&state.pool, application_id, environment_id).await?;
    require_application_read(&state.pool, p.user_id, team_id, application_id).await?;

    let rows: Vec<(
        Uuid,
        Uuid,
        String,
        String,
        DateTime<Utc>,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
        Option<String>,
        Option<String>,
        String,
        String,
        Option<DateTime<Utc>>,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        r#"SELECT j.id, j.application_id, j.status, j.log, j.created_at, j.started_at, j.finished_at,
                  j.git_ref, j.git_sha, COALESCE(j.job_kind, 'standard') AS job_kind,
                  COALESCE(NULLIF(trim(j.deploy_strategy), ''), 'standard') AS deploy_strategy,
                  j.approved_at,
                  j.log_object_key,
                  j.artifact_manifest_key
           FROM deploy_jobs j
           WHERE j.application_id = $1
           ORDER BY j.created_at DESC
           LIMIT 50"#,
    )
    .bind(application_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let mut out = Vec::new();
    for (
        id,
        aid,
        status_s,
        log,
        created_at,
        started_at,
        finished_at,
        git_ref,
        git_sha,
        job_kind,
        deploy_strategy,
        approved_at,
        log_object_key,
        artifact_manifest_key,
    ) in rows
    {
        let jk = job_kind.trim();
        let job_kind_opt = if jk == "standard" {
            None
        } else {
            Some(jk.to_string())
        };
        let ds = deploy_strategy.trim();
        let deploy_strategy_opt = if ds.is_empty() || ds == "standard" {
            None
        } else {
            Some(ds.to_string())
        };
        out.push(DeployJobSummary {
            id,
            application_id: aid,
            status: job_status_from_db(&status_s)?,
            log,
            created_at,
            started_at,
            finished_at,
            git_ref,
            git_sha,
            job_kind: job_kind_opt,
            deploy_strategy: deploy_strategy_opt,
            approved_at,
            log_object_key,
            artifact_manifest_key,
        });
    }
    Ok(Json(out))
}

/// SSE: `data: {"status","log","job_id"}` every ~1s until the job reaches a terminal status.
async fn stream_deploy_job_log(
    State(state): State<Arc<AppState>>,
    Path((team_id, job_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;

    let app_id: Option<Uuid> = sqlx::query_scalar(
        r#"SELECT j.application_id FROM deploy_jobs j
 JOIN applications a ON a.id = j.application_id
           JOIN environments e ON e.id = a.environment_id
           JOIN projects p ON p.id = e.project_id
           WHERE j.id = $1 AND p.team_id = $2"#,
    )
    .bind(job_id)
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some(application_id) = app_id else {
        return Err(ApiError::NotFound);
    };
    require_application_read(&state.pool, p.user_id, team_id, application_id).await?;

    let pool = state.pool.clone();
    let stream = async_stream::stream! {
        loop {
            let row: Option<(String, String)> = sqlx::query_as(
                r#"SELECT j.status::text, j.log FROM deploy_jobs j WHERE j.id = $1"#,
            )
            .bind(job_id)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

            let Some((status, log)) = row else {
                break;
            };
            let payload = serde_json::json!({
                "status": status,
                "job_id": job_id,
                "log": log,
            });
            let chunk = format!(
                "data: {}\n\n",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into())
            );
            yield Ok::<Bytes, std::convert::Infallible>(Bytes::from(chunk));
            if status == "succeeded" || status == "failed" {
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    };

    let body = Body::from_stream(stream);
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(body)
        .unwrap())
}

#[derive(Deserialize)]
struct ContainerLogQuery {
    #[serde(default)]
    pr: Option<i32>,
}

/// SSE: `data: {"container","log"}` every ~2s from `docker logs --tail 400` on the app destination.
async fn stream_application_container_log(
    State(state): State<Arc<AppState>>,
    Path((team_id, project_id, environment_id, application_id)): Path<(Uuid, Uuid, Uuid, Uuid)>,
    Query(q): Query<ContainerLogQuery>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    ensure_env_in_team_project(&state.pool, team_id, project_id, environment_id).await?;
    ensure_application_in_env(&state.pool, application_id, environment_id).await?;
    require_application_read(&state.pool, p.user_id, team_id, application_id).await?;

    let row: Option<(
        String,
        Option<Uuid>,
        String,
        Option<String>,
        Option<i32>,
        Option<String>,
        Option<Vec<u8>>,
    )> = sqlx::query_as(
        r#"SELECT a.slug, a.destination_id, COALESCE(d.kind, '') AS dest_kind,
                  s.host, s.ssh_port, s.ssh_user, s.ssh_private_key_ciphertext
           FROM applications a
           LEFT JOIN destinations d ON d.id = a.destination_id
           LEFT JOIN servers s ON s.id = d.server_id
           WHERE a.id = $1 AND a.environment_id = $2"#,
    )
    .bind(application_id)
    .bind(environment_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((slug, dest_id, dest_kind, host_o, port_o, user_o, cipher_o)) = row else {
        return Err(ApiError::NotFound);
    };

    let exec_target = match build_deploy_exec_from_destination(
        dest_id,
        &dest_kind,
        host_o,
        port_o,
        user_o,
        cipher_o,
        &state.deploy_worker,
    ) {
        Ok(e) => e,
        Err(msg) => return Err(ApiError::BadRequest(msg.into())),
    };

    let cname = if let Some(pr) = q.pr {
        preview_docker_container_name(&slug, application_id, pr)
    } else {
        let short = application_id.as_simple().to_string();
        let short8 = &short[..short.len().min(8)];
        let slug_part = docker_name_part(&slug);
        if slug_part.is_empty() {
            format!("dw-{}", short8)
        } else {
            format!("dw-{}-{}", slug_part, short8)
        }
    };

    let exec_clone = exec_target.clone();
    let cname_clone = cname.clone();
    let stream = async_stream::stream! {
        loop {
            let cmd = format!("docker logs --tail 400 {}", shell_single_quote(&cname_clone));
            let payload = match deploy_shell_exec(&exec_clone, &cmd).await {
                Ok(text) => serde_json::json!({ "container": cname_clone, "log": text }),
                Err(e) => serde_json::json!({ "container": cname_clone, "error": e }),
            };
            let chunk = format!(
                "data: {}\n\n",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into())
            );
            yield Ok::<Bytes, std::convert::Infallible>(Bytes::from(chunk));
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    };

    let body = Body::from_stream(stream);
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(body)
        .unwrap())
}

async fn get_deploy_job(
    State(state): State<Arc<AppState>>,
    Path((team_id, job_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<DeployJobSummary>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;

    let row: Option<(
        Uuid,
        Uuid,
        String,
        String,
        DateTime<Utc>,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
        Option<String>,
        Option<String>,
        String,
        String,
        Option<DateTime<Utc>>,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        r#"SELECT j.id, j.application_id, j.status, j.log, j.created_at, j.started_at, j.finished_at,
                  j.git_ref, j.git_sha, COALESCE(j.job_kind, 'standard') AS job_kind,
                  COALESCE(NULLIF(trim(j.deploy_strategy), ''), 'standard') AS deploy_strategy,
                  j.approved_at,
                  j.log_object_key,
                  j.artifact_manifest_key
           FROM deploy_jobs j
           JOIN applications a ON a.id = j.application_id
           JOIN environments e ON e.id = a.environment_id
           JOIN projects p ON p.id = e.project_id
           WHERE j.id = $1 AND p.team_id = $2"#,
    )
    .bind(job_id)
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((
        id,
        aid,
        status_s,
        log,
        created_at,
        started_at,
        finished_at,
        git_ref,
        git_sha,
        job_kind,
        deploy_strategy,
        approved_at,
        log_object_key,
        artifact_manifest_key,
    )) = row
    else {
        return Err(ApiError::NotFound);
    };

    require_application_read(&state.pool, p.user_id, team_id, aid).await?;

    let jk = job_kind.trim().to_string();
    let job_kind_opt = if jk == "standard" {
        None
    } else {
        Some(jk)
    };
    let ds = deploy_strategy.trim();
    let deploy_strategy_opt = if ds.is_empty() || ds == "standard" {
        None
    } else {
        Some(ds.to_string())
    };

    Ok(Json(DeployJobSummary {
        id,
        application_id: aid,
        status: job_status_from_db(&status_s)?,
        log,
        created_at,
        started_at,
        finished_at,
        git_ref,
        git_sha,
        job_kind: job_kind_opt,
        deploy_strategy: deploy_strategy_opt,
        approved_at,
        log_object_key,
        artifact_manifest_key,
    }))
}

fn parse_domains(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

async fn load_env_vars(
    pool: &sqlx::PgPool,
    application_id: Uuid,
    include_secret_values: bool,
) -> Result<Vec<ApplicationEnvVarPublic>, ApiError> {
    let rows: Vec<(String, String, bool)> = sqlx::query_as(
        "SELECT key, value, is_secret FROM application_env_vars WHERE application_id = $1 ORDER BY key",
    )
    .bind(application_id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(rows
        .into_iter()
        .map(|(key, value, is_secret)| ApplicationEnvVarPublic {
            key,
            value: if is_secret && !include_secret_values {
                None
            } else {
                Some(value)
            },
            is_secret,
        })
        .collect())
}

async fn allocate_auto_hostname(pool: &sqlx::PgPool, base_domain: &str) -> Result<String, ApiError> {
    let base = base_domain.trim().trim_end_matches('.').to_lowercase();
    if base.is_empty() {
        return Err(ApiError::Internal);
    }
    let chars = b"abcdefghijklmnopqrstuvwxyz0123456789";
    for _ in 0..48 {
        let sub: String = (0..10)
            .map(|_| chars[rand::thread_rng().gen_range(0..chars.len())] as char)
            .collect();
        let fq = format!("{sub}.{base}");
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM applications WHERE auto_hostname = $1)",
        )
        .bind(&fq)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
        if !exists {
            return Ok(fq);
        }
    }
    Err(ApiError::Internal)
}

#[derive(FromRow)]
struct ApplicationDbRow {
    id: Uuid,
    environment_id: Uuid,
    destination_id: Option<Uuid>,
    name: String,
    slug: String,
    docker_image: String,
    domains: serde_json::Value,
    git_repo_url: Option<String>,
    git_repo_full_name: Option<String>,
    auto_hostname: Option<String>,
    auto_deploy_on_push: bool,
    git_branch_pattern: String,
    build_image_from_git: bool,
    git_build_ref: String,
    dockerfile_path: String,
    pr_preview_enabled: bool,
    runtime_volumes_json: serde_json::Value,
    created_at: DateTime<Utc>,
    last_deployed_image: Option<String>,
    previous_deployed_image: Option<String>,
    deploy_strategy: String,
    require_deploy_approval: bool,
    pre_deploy_hook_url: Option<String>,
    post_deploy_hook_url: Option<String>,
}

async fn load_application_row(
    pool: &sqlx::PgPool,
    application_id: Uuid,
    environment_id: Uuid,
) -> Result<ApplicationDbRow, ApiError> {
    let row: Option<ApplicationDbRow> = sqlx::query_as(
        r#"SELECT id, environment_id, destination_id, name, slug, docker_image, domains, git_repo_url,
                  git_repo_full_name, auto_hostname, auto_deploy_on_push, git_branch_pattern,
                  build_image_from_git, git_build_ref, dockerfile_path, pr_preview_enabled,
                  COALESCE(runtime_volumes_json, '[]'::jsonb) AS runtime_volumes_json,
                  created_at,
                  last_deployed_image, previous_deployed_image, deploy_strategy, require_deploy_approval,
                  NULLIF(trim(pre_deploy_hook_url), '') AS pre_deploy_hook_url,
                  NULLIF(trim(post_deploy_hook_url), '') AS post_deploy_hook_url
           FROM applications WHERE id = $1 AND environment_id = $2"#,
    )
    .bind(application_id)
    .bind(environment_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    row.ok_or(ApiError::NotFound)
}

fn row_to_summary(row: &ApplicationDbRow) -> ApplicationSummary {
    ApplicationSummary {
        id: row.id,
        environment_id: row.environment_id,
        destination_id: row.destination_id,
        name: row.name.clone(),
        slug: row.slug.clone(),
        docker_image: row.docker_image.clone(),
        domains: parse_domains(&row.domains),
        auto_hostname: row.auto_hostname.clone(),
        git_repo_url: row.git_repo_url.clone(),
        git_repo_full_name: row.git_repo_full_name.clone(),
        auto_deploy_on_push: row.auto_deploy_on_push,
        git_branch_pattern: row.git_branch_pattern.clone(),
        build_image_from_git: row.build_image_from_git,
        git_build_ref: row.git_build_ref.clone(),
        dockerfile_path: row.dockerfile_path.clone(),
        pr_preview_enabled: row.pr_preview_enabled,
        created_at: row.created_at,
        last_deployed_image: row.last_deployed_image.clone(),
        previous_deployed_image: row.previous_deployed_image.clone(),
        deploy_strategy: row.deploy_strategy.clone(),
        require_deploy_approval: row.require_deploy_approval,
        pre_deploy_hook_url: row.pre_deploy_hook_url.clone(),
        post_deploy_hook_url: row.post_deploy_hook_url.clone(),
    }
}

async fn list_applications(
    State(state): State<Arc<AppState>>,
    Path((team_id, project_id, environment_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<Vec<ApplicationSummary>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    ensure_env_in_team_project(&state.pool, team_id, project_id, environment_id).await?;
    let full_list = require_team_access_read(&state.pool, p.user_id, team_id)
        .await
        .is_ok();
    if !full_list {
        require_some_app_membership_in_environment(&state.pool, p.user_id, environment_id).await?;
    }

    let rows: Vec<ApplicationDbRow> = sqlx::query_as(
        r#"SELECT id, environment_id, destination_id, name, slug, docker_image, domains, git_repo_url,
                  git_repo_full_name, auto_hostname, auto_deploy_on_push, git_branch_pattern,
                  build_image_from_git, git_build_ref, dockerfile_path, pr_preview_enabled,
                  COALESCE(runtime_volumes_json, '[]'::jsonb) AS runtime_volumes_json,
                  created_at,
                  last_deployed_image, previous_deployed_image, deploy_strategy, require_deploy_approval,
                  NULLIF(trim(pre_deploy_hook_url), '') AS pre_deploy_hook_url,
                  NULLIF(trim(post_deploy_hook_url), '') AS post_deploy_hook_url
           FROM applications WHERE environment_id = $1 ORDER BY name"#,
    )
    .bind(environment_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let out: Vec<ApplicationSummary> = if full_list {
        rows.iter().map(row_to_summary).collect()
    } else {
        let allowed: HashSet<Uuid> =
            application_ids_for_user_in_environment(&state.pool, p.user_id, environment_id)
                .await?
                .into_iter()
                .collect();
        rows.iter()
            .filter(|r| allowed.contains(&r.id))
            .map(row_to_summary)
            .collect()
    };
    Ok(Json(out))
}

#[derive(Deserialize)]
struct CreateApplicationBody {
    name: String,
    #[serde(default)]
    slug: Option<String>,
    docker_image: String,
    #[serde(default)]
    destination_id: Option<Uuid>,
    #[serde(default)]
    domains: Option<Vec<String>>,
    #[serde(default)]
    git_repo_url: Option<String>,
    /// If omitted, derived from `git_repo_url` when possible.
    #[serde(default)]
    git_repo_full_name: Option<String>,
    #[serde(default)]
    auto_deploy_on_push: bool,
    #[serde(default)]
    git_branch_pattern: Option<String>,
    #[serde(default)]
    build_image_from_git: bool,
    #[serde(default)]
    git_build_ref: Option<String>,
    #[serde(default)]
    dockerfile_path: Option<String>,
    #[serde(default)]
    pr_preview_enabled: bool,
    /// `standard` | `blue_green` | `canary` | `rolling`
    #[serde(default)]
    deploy_strategy: Option<String>,
    #[serde(default)]
    require_deploy_approval: Option<bool>,
    #[serde(default)]
    runtime_volumes: Option<Vec<RuntimeVolumeMount>>,
}

fn resolve_stored_git_full_name(
    explicit: Option<&str>,
    git_repo_url: Option<&str>,
) -> Option<String> {
    let from_explicit = explicit.map(str::trim).filter(|s| !s.is_empty()).and_then(|s| {
        normalize_github_repo_full_name(s).or_else(|| {
            if s.contains('/') {
                Some(normalize_git_remote_path(s))
            } else {
                None
            }
        })
    });
    from_explicit.or_else(|| {
        git_repo_url
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .and_then(normalize_github_repo_full_name)
    })
}

fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .fold(String::new(), |mut acc, c| {
            if acc.ends_with('-') && c == '-' {
                return acc;
            }
            acc.push(c);
            acc
        })
}

async fn create_application(
    State(state): State<Arc<AppState>>,
    Path((team_id, project_id, environment_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Json(body): Json<CreateApplicationBody>,
) -> Result<(StatusCode, Json<ApplicationSummary>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    ensure_env_in_team_project(&state.pool, team_id, project_id, environment_id).await?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;

    let name = body.name.trim();
    let img = body.docker_image.trim();
    if name.is_empty() || img.is_empty() {
        return Err(ApiError::BadRequest("name and docker_image required".into()));
    }
    let slug = body
        .slug
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| slugify(name));
    if slug.is_empty() {
        return Err(ApiError::BadRequest("invalid slug".into()));
    }

    if let Some(did) = body.destination_id {
        let ok: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM destinations WHERE id = $1 AND team_id = $2)",
        )
        .bind(did)
        .bind(team_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
        if !ok {
            return Err(ApiError::BadRequest("destination not on this team".into()));
        }
    }

    let id = Uuid::new_v4();
    let now = Utc::now();

    let auto_hostname = if let Some(ref base) = state.deploy_worker.apps_base_domain {
        Some(
            allocate_auto_hostname(&state.pool, base)
                .await
                .map_err(|_| ApiError::Internal)?,
        )
    } else {
        None
    };

    let domains_list: Vec<String> = if let Some(ref fq) = auto_hostname {
        if body.domains.as_ref().map(|d| d.is_empty()).unwrap_or(true) {
            vec![fq.clone()]
        } else {
            body.domains.clone().unwrap_or_default()
        }
    } else {
        body.domains.clone().unwrap_or_default()
    };
    let domains_json =
        serde_json::to_value(domains_list).unwrap_or_else(|_| serde_json::json!([]));

    let git_url = body
        .git_repo_url
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let git_full = resolve_stored_git_full_name(
        body.git_repo_full_name.as_deref(),
        git_url.as_deref(),
    );

    if body.auto_deploy_on_push && git_full.is_none() {
        return Err(ApiError::BadRequest("auto_deploy_on_push requires git_repo_full_name or a normalizable git_repo_url".into()));
    }

    if body.pr_preview_enabled && git_full.is_none() {
        return Err(ApiError::BadRequest(
            "pr_preview_enabled requires git_repo_full_name or a normalizable git_repo_url".into(),
        ));
    }

    let deploy_strategy = body
        .deploy_strategy
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "standard".to_string());
    if !matches!(
        deploy_strategy.as_str(),
        "standard" | "blue_green" | "canary" | "rolling"
    ) {
        return Err(ApiError::BadRequest(
            "deploy_strategy must be standard, blue_green, canary, or rolling".into(),
        ));
    }

    if body.build_image_from_git && git_url.is_none() {
        return Err(ApiError::BadRequest("build_image_from_git requires git_repo_url (clone URL on the destination host)".into()));
    }

    let branch_pat = body
        .git_branch_pattern
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("main")
        .to_string();

    let git_br = body
        .git_build_ref
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("main")
        .to_string();
    let docker_fp = body
        .dockerfile_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Dockerfile")
        .to_string();

    let require_deploy_approval = body.require_deploy_approval.unwrap_or(false);

    let runtime_volumes =
        validate_and_normalize_runtime_volumes(body.runtime_volumes.unwrap_or_default())?;
    let runtime_volumes_json =
        serde_json::to_value(&runtime_volumes).unwrap_or_else(|_| serde_json::json!([]));

    let r = sqlx::query(
        r#"INSERT INTO applications (id, environment_id, destination_id, name, slug, docker_image, domains, git_repo_url,
               git_repo_full_name, auto_hostname, auto_deploy_on_push, git_branch_pattern,
               build_image_from_git, git_build_ref, dockerfile_path, pr_preview_enabled, runtime_volumes_json, created_at,
               deploy_strategy, require_deploy_approval, pre_deploy_hook_url, post_deploy_hook_url)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, NULL, NULL)"#,
    )
    .bind(id)
    .bind(environment_id)
    .bind(body.destination_id)
    .bind(name)
    .bind(&slug)
    .bind(img)
    .bind(&domains_json)
    .bind(&git_url)
    .bind(&git_full)
    .bind(&auto_hostname)
    .bind(body.auto_deploy_on_push)
    .bind(&branch_pat)
    .bind(body.build_image_from_git)
    .bind(&git_br)
    .bind(&docker_fp)
    .bind(body.pr_preview_enabled)
    .bind(&runtime_volumes_json)
    .bind(now)
    .bind(&deploy_strategy)
    .bind(require_deploy_approval)
    .execute(&state.pool)
    .await;

    if r.is_err() {
        return Err(ApiError::Conflict("slug already exists in environment".into()));
    }

    let created = ApplicationDbRow {
        id,
        environment_id,
        destination_id: body.destination_id,
        name: name.to_string(),
        slug,
        docker_image: img.to_string(),
        domains: domains_json,
        git_repo_url: git_url,
        git_repo_full_name: git_full,
        auto_hostname,
        auto_deploy_on_push: body.auto_deploy_on_push,
        git_branch_pattern: branch_pat,
        build_image_from_git: body.build_image_from_git,
        git_build_ref: git_br,
        dockerfile_path: docker_fp,
        pr_preview_enabled: body.pr_preview_enabled,
        runtime_volumes_json,
        created_at: now,
        last_deployed_image: None,
        previous_deployed_image: None,
        deploy_strategy,
        require_deploy_approval,
        pre_deploy_hook_url: None,
        post_deploy_hook_url: None,
    };

    Ok((StatusCode::CREATED, Json(row_to_summary(&created))))
}

async fn get_application(
    State(state): State<Arc<AppState>>,
    Path((team_id, project_id, environment_id, application_id)): Path<(Uuid, Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<ApplicationDetail>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    ensure_env_in_team_project(&state.pool, team_id, project_id, environment_id).await?;
    require_application_read(&state.pool, p.user_id, team_id, application_id).await?;

    let row = load_application_row(&state.pool, application_id, environment_id).await?;
    let include_secrets =
        user_can_see_application_secrets(&state.pool, p.user_id, team_id, application_id).await?;
    let env_vars = load_env_vars(&state.pool, row.id, include_secrets).await?;
    let runtime_volumes: Vec<RuntimeVolumeMount> =
        serde_json::from_value(row.runtime_volumes_json.clone()).unwrap_or_default();

    Ok(Json(ApplicationDetail {
        application: row_to_summary(&row),
        env_vars,
        runtime_volumes,
    }))
}

#[derive(Deserialize)]
struct EnvVarInput {
    key: String,
    value: String,
    #[serde(default)]
    is_secret: bool,
}

fn normalize_optional_hook_url(raw: &str) -> Result<Option<String>, ApiError> {
    let t = raw.trim();
    if t.is_empty() {
        return Ok(None);
    }
    if !t.starts_with("http://") && !t.starts_with("https://") {
        return Err(ApiError::BadRequest(
            "deploy hook URL must start with http:// or https://".into(),
        ));
    }
    Ok(Some(t.to_string()))
}

#[derive(Deserialize)]
struct PatchApplicationBody {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    docker_image: Option<String>,
    #[serde(default)]
    destination_id: Option<Option<Uuid>>,
    #[serde(default)]
    domains: Option<Vec<String>>,
    #[serde(default)]
    git_repo_url: Option<Option<String>>,
    #[serde(default)]
    git_repo_full_name: Option<Option<String>>,
    #[serde(default)]
    auto_deploy_on_push: Option<bool>,
    #[serde(default)]
    git_branch_pattern: Option<String>,
    #[serde(default)]
    build_image_from_git: Option<bool>,
    #[serde(default)]
    git_build_ref: Option<String>,
    #[serde(default)]
    dockerfile_path: Option<String>,
    #[serde(default)]
    pr_preview_enabled: Option<bool>,
    #[serde(default)]
    deploy_strategy: Option<String>,
    #[serde(default)]
    require_deploy_approval: Option<bool>,
    #[serde(default)]
    pre_deploy_hook_url: Option<String>,
    #[serde(default)]
    post_deploy_hook_url: Option<String>,
    #[serde(default)]
    runtime_volumes: Option<Vec<RuntimeVolumeMount>>,
    /// When set, replaces all env vars for this application.
    #[serde(default)]
    env_vars: Option<Vec<EnvVarInput>>,
}

async fn update_application(
    State(state): State<Arc<AppState>>,
    Path((team_id, project_id, environment_id, application_id)): Path<(Uuid, Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Json(body): Json<PatchApplicationBody>,
) -> Result<Json<ApplicationDetail>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    ensure_env_in_team_project(&state.pool, team_id, project_id, environment_id).await?;
    require_application_mutate(&state.pool, p.user_id, team_id, application_id).await?;

    let mut r = load_application_row(&state.pool, application_id, environment_id).await?;

    if let Some(n) = body.name.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        r.name = n.to_string();
    }
    if let Some(img) = body.docker_image.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        r.docker_image = img.to_string();
    }
    if let Some(ref opt) = body.destination_id {
        if let Some(d) = opt {
            let ok: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM destinations WHERE id = $1 AND team_id = $2)",
            )
            .bind(d)
            .bind(team_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
            if !ok {
                return Err(ApiError::BadRequest("destination not on this team".into()));
            }
            r.destination_id = Some(*d);
        } else {
            r.destination_id = None;
        }
    }
    if let Some(dom) = body.domains {
        r.domains = serde_json::to_value(dom).unwrap_or_else(|_| serde_json::json!([]));
    }
    if let Some(g) = body.git_repo_url {
        r.git_repo_url = g.and_then(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        });
    }
    if let Some(g_opt) = body.git_repo_full_name {
        r.git_repo_full_name = match g_opt {
            None => None,
            Some(s) => {
                let t = s.trim();
                if t.is_empty() {
                    None
                } else {
                    normalize_github_repo_full_name(t).or_else(|| {
                        if t.contains('/') {
                            Some(normalize_git_remote_path(t))
                        } else {
                            None
                        }
                    })
                }
            }
        };
    }
    if let Some(ad) = body.auto_deploy_on_push {
        r.auto_deploy_on_push = ad;
    }
    if let Some(ref p) = body.git_branch_pattern {
        let t = p.trim();
        if !t.is_empty() {
            r.git_branch_pattern = t.to_string();
        }
    }
    if let Some(b) = body.build_image_from_git {
        r.build_image_from_git = b;
    }
    if let Some(ref p) = body.git_build_ref {
        let t = p.trim();
        if !t.is_empty() {
            r.git_build_ref = t.to_string();
        }
    }
    if let Some(ref p) = body.dockerfile_path {
        let t = p.trim();
        if !t.is_empty() {
            r.dockerfile_path = t.to_string();
        }
    }
    if let Some(pp) = body.pr_preview_enabled {
        r.pr_preview_enabled = pp;
    }
    if let Some(ref ds) = body.deploy_strategy {
        let t = ds.trim();
        if !t.is_empty() {
            if !matches!(t, "standard" | "blue_green" | "canary" | "rolling") {
                return Err(ApiError::BadRequest(
                    "deploy_strategy must be standard, blue_green, canary, or rolling".into(),
                ));
            }
            r.deploy_strategy = t.to_string();
        }
    }
    if let Some(v) = body.require_deploy_approval {
        r.require_deploy_approval = v;
    }
    if let Some(ref u) = body.pre_deploy_hook_url {
        r.pre_deploy_hook_url = normalize_optional_hook_url(u)?;
    }
    if let Some(ref u) = body.post_deploy_hook_url {
        r.post_deploy_hook_url = normalize_optional_hook_url(u)?;
    }
    if let Some(vols) = body.runtime_volumes {
        let vols = validate_and_normalize_runtime_volumes(vols)?;
        r.runtime_volumes_json = serde_json::to_value(&vols).unwrap_or_else(|_| serde_json::json!([]));
    }

    if r.build_image_from_git && r.git_repo_url.is_none() {
        return Err(ApiError::BadRequest("build_image_from_git requires git_repo_url".into()));
    }

    if r.git_repo_full_name.is_none() {
        r.git_repo_full_name = resolve_stored_git_full_name(None, r.git_repo_url.as_deref());
    }

    if r.auto_deploy_on_push && r.git_repo_full_name.is_none() {
        return Err(ApiError::BadRequest("auto_deploy_on_push requires git_repo_full_name or a normalizable git_repo_url".into()));
    }

    if r.pr_preview_enabled && r.git_repo_full_name.is_none() {
        return Err(ApiError::BadRequest(
            "pr_preview_enabled requires git_repo_full_name or a normalizable git_repo_url".into(),
        ));
    }

    sqlx::query(
        r#"UPDATE applications SET name = $1, docker_image = $2, destination_id = $3, domains = $4, git_repo_url = $5,
               git_repo_full_name = $6, auto_deploy_on_push = $7, git_branch_pattern = $8,
               build_image_from_git = $9, git_build_ref = $10, dockerfile_path = $11, pr_preview_enabled = $12,
               deploy_strategy = $13, require_deploy_approval = $14, runtime_volumes_json = $15,
               pre_deploy_hook_url = $16, post_deploy_hook_url = $17
           WHERE id = $18 AND environment_id = $19"#,
    )
    .bind(&r.name)
    .bind(&r.docker_image)
    .bind(r.destination_id)
    .bind(&r.domains)
    .bind(&r.git_repo_url)
    .bind(&r.git_repo_full_name)
    .bind(r.auto_deploy_on_push)
    .bind(&r.git_branch_pattern)
    .bind(r.build_image_from_git)
    .bind(&r.git_build_ref)
    .bind(&r.dockerfile_path)
    .bind(r.pr_preview_enabled)
    .bind(&r.deploy_strategy)
    .bind(r.require_deploy_approval)
    .bind(&r.runtime_volumes_json)
    .bind(&r.pre_deploy_hook_url)
    .bind(&r.post_deploy_hook_url)
    .bind(application_id)
    .bind(environment_id)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    if let Some(vars) = body.env_vars {
        let mut tx = state.pool.begin().await.map_err(|_| ApiError::Internal)?;
        sqlx::query("DELETE FROM application_env_vars WHERE application_id = $1")
            .bind(application_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal)?;
        for v in vars {
            let k = v.key.trim();
            if k.is_empty() {
                continue;
            }
            let vid = Uuid::new_v4();
            let now = Utc::now();
            sqlx::query(
                r#"INSERT INTO application_env_vars (id, application_id, key, value, is_secret, created_at)
                   VALUES ($1, $2, $3, $4, $5, $6)"#,
            )
            .bind(vid)
            .bind(application_id)
            .bind(k)
            .bind(&v.value)
            .bind(v.is_secret)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::Conflict("duplicate env key".into()))?;
        }
        tx.commit().await.map_err(|_| ApiError::Internal)?;
    }

    let env_vars = load_env_vars(&state.pool, application_id, true).await?;
    let runtime_volumes: Vec<RuntimeVolumeMount> =
        serde_json::from_value(r.runtime_volumes_json.clone()).unwrap_or_default();
    Ok(Json(ApplicationDetail {
        application: row_to_summary(&r),
        env_vars,
        runtime_volumes,
    }))
}

async fn delete_application(
    State(state): State<Arc<AppState>>,
    Path((team_id, project_id, environment_id, application_id)): Path<(Uuid, Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    ensure_env_in_team_project(&state.pool, team_id, project_id, environment_id).await?;
    require_application_mutate(&state.pool, p.user_id, team_id, application_id).await?;

    let ip = crate::auth::peer_ip_from_headers(&headers).map(|i| i.to_string());
    try_log_team_audit(
        &state.pool,
        team_id,
        p.user_id,
        "application_deleted",
        "application",
        Some(application_id),
        serde_json::json!({ "environment_id": environment_id }),
        ip,
    )
    .await;

    let n = sqlx::query("DELETE FROM applications WHERE id = $1 AND environment_id = $2")
        .bind(application_id)
        .bind(environment_id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?
        .rows_affected();

    if n == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DeployWorkerConfig;

    #[test]
    fn docker_env_name_valid_cases() {
        assert!(docker_env_name_valid("FOO"));
        assert!(docker_env_name_valid("_X"));
        assert!(!docker_env_name_valid(""));
        assert!(!docker_env_name_valid("9A"));
        assert!(!docker_env_name_valid("NO-DASH"));
    }

    #[test]
    fn shell_single_quote_cases() {
        assert_eq!(shell_single_quote("abc"), "'abc'");
        assert_eq!(shell_single_quote("a'b"), "'a'\"'\"'b'");
        assert_eq!(shell_single_quote(""), "''");
    }

    #[test]
    fn normalize_git_remote_path_trims() {
        assert_eq!(
            normalize_git_remote_path("  MyGroup/MyProj.git/  "),
            "mygroup/myproj"
        );
    }

    #[test]
    fn source_commit_and_compare_urls() {
        assert_eq!(
            source_commit_page_url(Some("o/r"), Some("abcdef1234567")),
            Some("https://github.com/o/r/commit/abcdef1234567".into())
        );
        assert_eq!(
            source_commit_page_url(Some("g/sub/p"), Some("abcdef1234567")),
            Some("https://gitlab.com/g/sub/p/-/commit/abcdef1234567".into())
        );
        assert_eq!(
            source_compare_url(
                Some("o/r"),
                Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
                Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
            ),
            Some(
                "https://github.com/o/r/compare/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa...bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .into()
            )
        );
    }

    #[test]
    fn validate_env_for_docker_run_rejects_too_many() {
        let rows: Vec<(String, String, bool)> = (0..MAX_DOCKER_ENV_PAIRS + 3)
            .map(|i| (format!("K{i}"), "v".into(), false))
            .collect();
        assert!(validate_env_for_docker_run(&rows).is_err());
    }

    #[test]
    fn docker_run_cmd_includes_isolated_network_before_traefik() {
        let cfg = DeployWorkerConfig {
            server_key_encryption_key: [0u8; 32],
            platform_docker_enabled: false,
            apps_base_domain: None,
            git_cache_root: "/tmp/git-cache".into(),
            volumes_root: "/tmp/volumes".into(),
            edge_mode: "traefik".into(),
            traefik_docker_network: "edge_net".into(),
            app_container_port: 8080,
            pr_preview_isolated_network: false,
            smtp_settings: None,
        };
        let (full, _) = docker_run_cmd(
            "c1",
            "img:latest",
            &cfg,
            Some("pr1.example.com"),
            &[],
            &[],
            "/tmp/volumes/teams/t/apps/a",
            Some("dwpr-net"),
        );
        assert!(full.contains("--network 'dwpr-net'"));
        assert!(full.contains("--network 'edge_net'"));
        assert!(full.find("--network 'dwpr-net'") < full.find("--network 'edge_net'"));
    }
}
