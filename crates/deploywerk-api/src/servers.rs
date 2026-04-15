//! Team-scoped servers (SSH credentials encrypted at rest) and validation.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::extract::Query;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use deploywerk_core::{ServerStatus, ServerSummary};
use russh::keys::decode_secret_key;
use russh::{client, ChannelMsg, Disconnect};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::require_principal;
use crate::audit::try_log_team_audit;
use crate::crypto_keys::{decrypt_private_key, encrypt_private_key, KeyCryptoError};
use crate::error::ApiError;
use crate::rbac::{require_team_access_mutate, require_team_access_read};
use crate::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/v1/teams/{team_id}/servers",
            get(list_servers).post(create_server),
        )
        .route(
            "/api/v1/teams/{team_id}/servers/{server_id}",
            get(get_server).patch(update_server).delete(delete_server),
        )
        .route(
            "/api/v1/teams/{team_id}/servers/{server_id}/validate",
            post(validate_server),
        )
        .route(
            "/api/v1/teams/{team_id}/servers/{server_id}/docker/containers",
            get(docker_list_containers),
        )
        .route(
            "/api/v1/teams/{team_id}/servers/{server_id}/docker/containers/{container_ref}/inspect",
            get(docker_inspect_container),
        )
        .route(
            "/api/v1/teams/{team_id}/servers/{server_id}/docker/containers/{container_ref}/logs",
            get(docker_container_logs),
        )
        .route(
            "/api/v1/teams/{team_id}/servers/{server_id}/docker/containers/{container_ref}/start",
            post(docker_container_start),
        )
        .route(
            "/api/v1/teams/{team_id}/servers/{server_id}/docker/containers/{container_ref}/stop",
            post(docker_container_stop),
        )
        .route(
            "/api/v1/teams/{team_id}/servers/{server_id}/docker/containers/{container_ref}/restart",
            post(docker_container_restart),
        )
        .route(
            "/api/v1/teams/{team_id}/servers/{server_id}/docker/containers/{container_ref}/exec",
            post(docker_container_exec),
        )
}

fn shell_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}

fn status_from_db(s: &str) -> Result<ServerStatus, ApiError> {
    ServerStatus::parse(s).ok_or(ApiError::Internal)
}

async fn list_servers(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<ServerSummary>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_access_read(&state.pool, p.user_id, team_id).await?;

    let rows: Vec<(Uuid, Uuid, String, String, i32, String, String, Option<chrono::DateTime<Utc>>, Option<String>, chrono::DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT id, team_id, name, host, ssh_port, ssh_user, status, last_validated_at, last_validation_error, created_at
           FROM servers WHERE team_id = $1 ORDER BY name"#,
    )
    .bind(team_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let mut out = Vec::new();
    for (id, tid, name, host, ssh_port, ssh_user, status_s, last_validated_at, last_validation_error, created_at) in rows {
        out.push(ServerSummary {
            id,
            team_id: tid,
            name,
            host,
            ssh_port,
            ssh_user,
            status: status_from_db(&status_s)?,
            last_validated_at,
            last_validation_error,
            created_at,
        });
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
struct CreateServerBody {
    name: String,
    host: String,
    #[serde(default = "default_ssh_port")]
    ssh_port: i32,
    ssh_user: String,
    /// PEM or OpenSSH private key (plaintext over TLS only).
    ssh_private_key_pem: String,
}

fn default_ssh_port() -> i32 {
    22
}

async fn create_server(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<CreateServerBody>,
) -> Result<(StatusCode, Json<ServerSummary>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;

    let name = body.name.trim();
    let host = body.host.trim();
    let user = body.ssh_user.trim();
    if name.is_empty() || host.is_empty() || user.is_empty() {
        return Err(ApiError::BadRequest("name, host, and ssh_user required".into()));
    }
    if body.ssh_port <= 0 || body.ssh_port > 65535 {
        return Err(ApiError::BadRequest("invalid ssh_port".into()));
    }
    let key_pem = body.ssh_private_key_pem.trim();
    if key_pem.is_empty() {
        return Err(ApiError::BadRequest("ssh_private_key_pem required".into()));
    }

    let ciphertext = encrypt_private_key(&state.server_key_encryption_key, key_pem.as_bytes())
        .map_err(|e| match e {
            KeyCryptoError::Encrypt => ApiError::Internal,
            _ => ApiError::Internal,
        })?;

    let id = Uuid::new_v4();
    let now = Utc::now();
    let status = ServerStatus::Pending.as_str();

    sqlx::query(
        r#"INSERT INTO servers (id, team_id, name, host, ssh_port, ssh_user, ssh_private_key_ciphertext, status, last_validated_at, last_validation_error, created_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, NULL, $9)"#,
    )
    .bind(id)
    .bind(team_id)
    .bind(name)
    .bind(host)
    .bind(body.ssh_port)
    .bind(user)
    .bind(&ciphertext)
    .bind(status)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok((
        StatusCode::CREATED,
        Json(ServerSummary {
            id,
            team_id,
            name: name.to_string(),
            host: host.to_string(),
            ssh_port: body.ssh_port,
            ssh_user: user.to_string(),
            status: ServerStatus::Pending,
            last_validated_at: None,
            last_validation_error: None,
            created_at: now,
        }),
    ))
}

async fn get_server(
    State(state): State<Arc<AppState>>,
    Path((team_id, server_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<ServerSummary>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_access_read(&state.pool, p.user_id, team_id).await?;

    let row: Option<(Uuid, Uuid, String, String, i32, String, String, Option<chrono::DateTime<Utc>>, Option<String>, chrono::DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT id, team_id, name, host, ssh_port, ssh_user, status, last_validated_at, last_validation_error, created_at
           FROM servers WHERE id = $1 AND team_id = $2"#,
    )
    .bind(server_id)
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((id, tid, name, host, ssh_port, ssh_user, status_s, last_validated_at, last_validation_error, created_at)) =
        row
    else {
        return Err(ApiError::NotFound);
    };

    Ok(Json(ServerSummary {
        id,
        team_id: tid,
        name,
        host,
        ssh_port,
        ssh_user,
        status: status_from_db(&status_s)?,
        last_validated_at,
        last_validation_error,
        created_at,
    }))
}

#[derive(Deserialize)]
struct UpdateServerBody {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    ssh_port: Option<i32>,
    #[serde(default)]
    ssh_user: Option<String>,
    /// If set, replaces stored key (re-encrypted).
    #[serde(default)]
    ssh_private_key_pem: Option<String>,
}

async fn update_server(
    State(state): State<Arc<AppState>>,
    Path((team_id, server_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(body): Json<UpdateServerBody>,
) -> Result<Json<ServerSummary>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;

    let row: Option<(Uuid, Uuid, String, String, i32, String, Vec<u8>, String, Option<chrono::DateTime<Utc>>, Option<String>, chrono::DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT id, team_id, name, host, ssh_port, ssh_user, ssh_private_key_ciphertext, status, last_validated_at, last_validation_error, created_at
           FROM servers WHERE id = $1 AND team_id = $2"#,
    )
    .bind(server_id)
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((id, tid, mut name, mut host, mut ssh_port, mut ssh_user, mut ciphertext, status_s, last_validated_at, last_validation_error, created_at)) =
        row
    else {
        return Err(ApiError::NotFound);
    };

    if let Some(n) = body.name.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        name = n.to_string();
    }
    if let Some(h) = body.host.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        host = h.to_string();
    }
    if let Some(port) = body.ssh_port {
        if port <= 0 || port > 65535 {
            return Err(ApiError::BadRequest("invalid ssh_port".into()));
        }
        ssh_port = port;
    }
    if let Some(u) = body.ssh_user.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        ssh_user = u.to_string();
    }
    if let Some(ref pem) = body.ssh_private_key_pem {
        let key_pem = pem.trim();
        if key_pem.is_empty() {
            return Err(ApiError::BadRequest("empty ssh_private_key_pem".into()));
        }
        ciphertext = encrypt_private_key(&state.server_key_encryption_key, key_pem.as_bytes())
            .map_err(|_| ApiError::Internal)?;
        try_log_team_audit(
            &state.pool,
            team_id,
            p.user_id,
            "server.ssh_key.rotate",
            "server",
            Some(server_id),
            serde_json::json!({ "server_id": server_id }),
            None,
        )
        .await;
    }

    sqlx::query(
        r#"UPDATE servers SET name = $1, host = $2, ssh_port = $3, ssh_user = $4, ssh_private_key_ciphertext = $5
           WHERE id = $6 AND team_id = $7"#,
    )
    .bind(&name)
    .bind(&host)
    .bind(ssh_port)
    .bind(&ssh_user)
    .bind(&ciphertext)
    .bind(server_id)
    .bind(team_id)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(ServerSummary {
        id,
        team_id: tid,
        name,
        host,
        ssh_port,
        ssh_user,
        status: status_from_db(&status_s)?,
        last_validated_at,
        last_validation_error,
        created_at,
    }))
}

async fn delete_server(
    State(state): State<Arc<AppState>>,
    Path((team_id, server_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;

    let n = sqlx::query("DELETE FROM servers WHERE id = $1 AND team_id = $2")
        .bind(server_id)
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

#[derive(Serialize)]
pub struct ValidateServerResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

struct SshValidateClient;

impl client::Handler for SshValidateClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

/// Run a non-interactive remote command over SSH (single session). Returns combined stdout/stderr on success (exit 0).
pub(crate) async fn run_ssh_exec(
    host: String,
    port: i32,
    user: String,
    key_pem: String,
    command: &str,
) -> Result<String, String> {
    let port_u = u16::try_from(port).map_err(|_| "invalid ssh_port".to_string())?;
    let key = decode_secret_key(&key_pem, None).map_err(|e| format!("private key: {e}"))?;
    let key = Arc::new(key);

    let config = Arc::new(client::Config::default());
    let mut handle = client::connect(config, (host.as_str(), port_u), SshValidateClient)
        .await
        .map_err(|e| format!("connect: {e}"))?;

    let rsa_hash = handle
        .best_supported_rsa_hash()
        .await
        .map_err(|e| format!("key negotiation: {e}"))?;

    let auth = handle
        .authenticate_publickey(
            user,
            russh::keys::PrivateKeyWithHashAlg::new(Arc::clone(&key), rsa_hash.flatten()),
        )
        .await
        .map_err(|e| format!("authenticate: {e}"))?;

    if !auth.success() {
        return Err("SSH public key authentication failed".into());
    }

    let mut channel = handle
        .channel_open_session()
        .await
        .map_err(|e| format!("open session: {e}"))?;

    channel
        .exec(true, command)
        .await
        .map_err(|e| format!("exec: {e}"))?;

    let mut code: Option<u32> = None;
    let mut stderr = Vec::new();
    let mut stdout = Vec::new();
    loop {
        let Some(msg) = channel.wait().await else {
            break;
        };
        match msg {
            ChannelMsg::Data { data } => stdout.extend_from_slice(data.as_ref()),
            ChannelMsg::ExtendedData { data, .. } => stderr.extend_from_slice(data.as_ref()),
            ChannelMsg::ExitStatus { exit_status } => code = Some(exit_status),
            _ => {}
        }
    }

    let _ = handle
        .disconnect(Disconnect::ByApplication, "", "English")
        .await;

    let status = code.unwrap_or(u32::MAX);
    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&stderr)
    );
    if status != 0 {
        let short = out.chars().take(2000).collect::<String>();
        return Err(format!("exit {status}: {short}"));
    }
    Ok(out)
}

/// Run a shell command on the API host (platform Docker). Unix only; Windows dev returns an error.
pub(crate) async fn run_local_sh(command: &str) -> Result<String, String> {
    #[cfg(unix)]
    {
        use std::process::Stdio;
        let out = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("spawn: {e}"))?;
        let status = out.status.code().unwrap_or(-1);
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        if status != 0 {
            let short = combined.chars().take(2000).collect::<String>();
            return Err(format!("exit {status}: {short}"));
        }
        Ok(combined)
    }
    #[cfg(not(unix))]
    {
        let _ = command;
        Err("platform docker requires a Unix host with sh".into())
    }
}

async fn run_ssh_docker_check(
    host: String,
    port: i32,
    user: String,
    key_pem: String,
) -> Result<(), String> {
    run_ssh_exec(host, port, user, key_pem, "docker version")
        .await
        .map(|_| ())
}

async fn validate_server(
    State(state): State<Arc<AppState>>,
    Path((team_id, server_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<ValidateServerResponse>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;

    let row: Option<(String, i32, String, String, Vec<u8>)> = sqlx::query_as(
        "SELECT host, ssh_port, ssh_user, status, ssh_private_key_ciphertext FROM servers WHERE id = $1 AND team_id = $2",
    )
    .bind(server_id)
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((host, ssh_port, ssh_user, _status, ciphertext)) = row else {
        return Err(ApiError::NotFound);
    };

    let key_bytes = decrypt_private_key(&state.server_key_encryption_key, &ciphertext)
        .map_err(|_| ApiError::Internal)?;
    let key_pem = String::from_utf8(key_bytes).map_err(|_| ApiError::Internal)?;

    let res = run_ssh_docker_check(host, ssh_port, ssh_user, key_pem).await;

    let now = Utc::now();
    match res {
        Ok(()) => {
            sqlx::query(
                r#"UPDATE servers SET status = 'ready', last_validated_at = $1, last_validation_error = NULL WHERE id = $2 AND team_id = $3"#,
            )
            .bind(now)
            .bind(server_id)
            .bind(team_id)
            .execute(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
            Ok(Json(ValidateServerResponse {
                ok: true,
                detail: Some("docker version succeeded".into()),
            }))
        }
        Err(msg) => {
            let short = msg.chars().take(2000).collect::<String>();
            sqlx::query(
                r#"UPDATE servers SET status = 'error', last_validated_at = $1, last_validation_error = $2 WHERE id = $3 AND team_id = $4"#,
            )
            .bind(now)
            .bind(&short)
            .bind(server_id)
            .bind(team_id)
            .execute(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
            Ok(Json(ValidateServerResponse {
                ok: false,
                detail: Some(short),
            }))
        }
    }
}

fn docker_container_ref_valid(t: &str) -> bool {
    let t = t.trim();
    !t.is_empty()
        && t.len() <= 256
        && !t.chars().any(|c| {
            c.is_whitespace()
                || matches!(
                    c,
                    ';' | '|' | '&' | '$' | '`' | '(' | ')' | '<' | '>' | '\n' | '\r' | '\'' | '"'
                )
        })
}

async fn ssh_exec_on_server(
    state: &AppState,
    team_id: Uuid,
    server_id: Uuid,
    command: &str,
) -> Result<String, ApiError> {
    let row: Option<(String, i32, String, Vec<u8>)> = sqlx::query_as(
        "SELECT host, ssh_port, ssh_user, ssh_private_key_ciphertext FROM servers WHERE id = $1 AND team_id = $2",
    )
    .bind(server_id)
    .bind(team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let Some((host, ssh_port, ssh_user, ciphertext)) = row else {
        return Err(ApiError::NotFound);
    };

    let key_bytes = decrypt_private_key(&state.server_key_encryption_key, &ciphertext)
        .map_err(|_| ApiError::Internal)?;
    let key_pem = String::from_utf8(key_bytes).map_err(|_| ApiError::Internal)?;

    run_ssh_exec(host, ssh_port, ssh_user, key_pem, command)
        .await
        .map_err(|e| ApiError::BadRequest(e))
}

#[derive(Serialize)]
struct DockerContainerRow {
    id: String,
    name: String,
    status: String,
    image: String,
}

async fn docker_list_containers(
    State(state): State<Arc<AppState>>,
    Path((team_id, server_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<Vec<DockerContainerRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_access_read(&state.pool, p.user_id, team_id).await?;

    let cmd = "docker ps -a --no-trunc --format '{{.ID}}\t{{.Names}}\t{{.Status}}\t{{.Image}}'";
    let out = ssh_exec_on_server(&state, team_id, server_id, cmd).await?;
    let mut rows = Vec::new();
    for line in out.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 4 {
            rows.push(DockerContainerRow {
                id: parts[0].to_string(),
                name: parts[1].to_string(),
                status: parts[2].to_string(),
                image: parts[3].to_string(),
            });
        }
    }
    Ok(Json(rows))
}

async fn docker_inspect_container(
    State(state): State<Arc<AppState>>,
    Path((team_id, server_id, container_ref)): Path<(Uuid, Uuid, String)>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_access_read(&state.pool, p.user_id, team_id).await?;
    if !docker_container_ref_valid(&container_ref) {
        return Err(ApiError::BadRequest("invalid container reference".into()));
    }
    let cmd = format!("docker inspect {}", shell_single_quote(&container_ref));
    let out = ssh_exec_on_server(&state, team_id, server_id, &cmd).await?;
    let v: serde_json::Value =
        serde_json::from_str(out.trim()).map_err(|_| ApiError::BadRequest("invalid inspect json".into()))?;
    Ok(Json(v))
}

#[derive(Deserialize)]
struct DockerLogsQuery {
    #[serde(default = "default_logs_tail")]
    tail: u32,
}

fn default_logs_tail() -> u32 {
    200
}

async fn docker_container_logs(
    State(state): State<Arc<AppState>>,
    Path((team_id, server_id, container_ref)): Path<(Uuid, Uuid, String)>,
    Query(q): Query<DockerLogsQuery>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_access_read(&state.pool, p.user_id, team_id).await?;
    if !docker_container_ref_valid(&container_ref) {
        return Err(ApiError::BadRequest("invalid container reference".into()));
    }
    let tail = q.tail.clamp(1, 10_000);
    let cmd = format!(
        "docker logs --tail {} {}",
        tail,
        shell_single_quote(&container_ref)
    );
    let out = ssh_exec_on_server(&state, team_id, server_id, &cmd).await?;
    Ok(Json(serde_json::json!({ "log": out })))
}

async fn docker_container_start(
    State(state): State<Arc<AppState>>,
    Path((team_id, server_id, container_ref)): Path<(Uuid, Uuid, String)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;
    if !docker_container_ref_valid(&container_ref) {
        return Err(ApiError::BadRequest("invalid container reference".into()));
    }
    let cmd = format!("docker start {}", shell_single_quote(&container_ref));
    ssh_exec_on_server(&state, team_id, server_id, &cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn docker_container_stop(
    State(state): State<Arc<AppState>>,
    Path((team_id, server_id, container_ref)): Path<(Uuid, Uuid, String)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;
    if !docker_container_ref_valid(&container_ref) {
        return Err(ApiError::BadRequest("invalid container reference".into()));
    }
    let cmd = format!("docker stop {}", shell_single_quote(&container_ref));
    ssh_exec_on_server(&state, team_id, server_id, &cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn docker_container_restart(
    State(state): State<Arc<AppState>>,
    Path((team_id, server_id, container_ref)): Path<(Uuid, Uuid, String)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;
    if !docker_container_ref_valid(&container_ref) {
        return Err(ApiError::BadRequest("invalid container reference".into()));
    }
    let cmd = format!("docker restart {}", shell_single_quote(&container_ref));
    ssh_exec_on_server(&state, team_id, server_id, &cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct DockerExecBody {
    argv: Vec<String>,
}

fn validate_exec_argv(argv: &[String]) -> Result<(), ApiError> {
    if argv.is_empty() || argv.len() > 32 {
        return Err(ApiError::BadRequest(
            "argv must have 1..=32 elements".into(),
        ));
    }
    for a in argv {
        if a.is_empty() || a.len() > 512 {
            return Err(ApiError::BadRequest("invalid argv element".into()));
        }
        if a.chars()
            .any(|c| c == '\n' || c == '\r' || c == '\'' || c == '`')
        {
            return Err(ApiError::BadRequest("invalid argv character".into()));
        }
    }
    Ok(())
}

async fn docker_container_exec(
    State(state): State<Arc<AppState>>,
    Path((team_id, server_id, container_ref)): Path<(Uuid, Uuid, String)>,
    headers: HeaderMap,
    Json(body): Json<DockerExecBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    require_team_access_mutate(&state.pool, p.user_id, team_id).await?;
    if !docker_container_ref_valid(&container_ref) {
        return Err(ApiError::BadRequest("invalid container reference".into()));
    }
    validate_exec_argv(&body.argv)?;
    let mut cmd = format!("docker exec {}", shell_single_quote(&container_ref));
    for a in &body.argv {
        cmd.push(' ');
        cmd.push_str(&shell_single_quote(a));
    }
    let out = ssh_exec_on_server(&state, team_id, server_id, &cmd).await?;
    Ok(Json(serde_json::json!({ "output": out })))
}
