//! Allowlisted `deploywerk`-style commands for the web CLI: `POST /api/v1/teams/{team_id}/cli/invoke`.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::post;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::applications::enqueue_deploy_application_in_team;
use crate::audit::try_log_team_audit;
use crate::auth::{require_principal, Principal};
use crate::error::ApiError;
use crate::rbac::require_team_access_read;
use crate::AppState;

pub fn routes() -> axum::Router<Arc<AppState>> {
    axum::Router::new().route(
        "/api/v1/teams/{team_id}/cli/invoke",
        post(cli_invoke),
    )
}

#[derive(Deserialize)]
pub struct CliInvokeBody {
    pub command_line: String,
}

#[derive(Serialize)]
pub struct CliInvokeResponse {
    pub exit_code: i32,
    pub stdout: String,
}

async fn cli_invoke(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<CliInvokeBody>,
) -> Result<Json<CliInvokeResponse>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    require_team_access_read(&state.pool, p.user_id, team_id).await?;

    let line = body.command_line.trim().to_string();
    let parts: Vec<&str> = line.split_whitespace().collect();

    let (stdout, exit_code) = dispatch_cli(state.clone(), &p, team_id, parts.as_slice()).await?;

    let ip = crate::auth::peer_ip_from_headers(&headers).map(|i| i.to_string());
    try_log_team_audit(
        &state.pool,
        team_id,
        p.user_id,
        "cli.invoke",
        "cli",
        None,
        serde_json::json!({
            "command_line": line,
            "exit_code": exit_code,
        }),
        ip,
    )
    .await;

    Ok(Json(CliInvokeResponse { exit_code, stdout }))
}

fn help_text() -> String {
    r#"DeployWerk web CLI (allowlisted). Commands:
  help
  whoami
  teams list              — teams you belong to
  projects list            — projects in this team
  environments list <project_id>
  applications list <project_id> <environment_id>
  tokens list              — your API tokens (names only)
  servers list             — SSH servers in this team
  deploy <application_id>  — enqueue deploy (requires deploy scope)

Examples:
  projects list
  deploy 550e8400-e29b-41d4-a716-446655440000
"#
    .to_string()
}

async fn dispatch_cli(
    state: Arc<AppState>,
    p: &Principal,
    team_id: Uuid,
    parts: &[&str],
) -> Result<(String, i32), ApiError> {
    match parts {
        [] | ["help"] | ["help", ..] => Ok((help_text(), 0)),

        ["whoami"] => {
            let row: Option<(String, Option<String>)> = sqlx::query_as(
                "SELECT email, name FROM users WHERE id = $1",
            )
            .bind(p.user_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
            let Some((email, name)) = row else {
                return Ok(("user not found\n".into(), 1));
            };
            let mut s = format!("email: {email}\n");
            if let Some(n) = name {
                s.push_str(&format!("name: {n}\n"));
            }
            s.push_str(&format!("session: {}\n", if p.is_jwt { "jwt" } else { "api_token" }));
            Ok((s, 0))
        }

        ["teams"] | ["teams", "list"] => {
            let rows: Vec<(Uuid, String, String, String)> = sqlx::query_as(
                r#"SELECT t.id, t.name, t.slug, tm.role
                   FROM teams t
                   INNER JOIN team_memberships tm ON tm.team_id = t.id AND tm.user_id = $1
                   ORDER BY t.name"#,
            )
            .bind(p.user_id)
            .fetch_all(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
            if rows.is_empty() {
                return Ok(("No teams.\n".into(), 0));
            }
            let mut s = String::from("id\tname\tslug\trole\n");
            for (id, name, slug, role) in rows {
                s.push_str(&format!("{id}\t{name}\t{slug}\t{role}\n"));
            }
            Ok((s, 0))
        }

        ["projects"] | ["projects", "list"] => {
            let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
                "SELECT id, name, slug FROM projects WHERE team_id = $1 ORDER BY name",
            )
            .bind(team_id)
            .fetch_all(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
            if rows.is_empty() {
                return Ok(("No projects.\n".into(), 0));
            }
            let mut s = String::from("id\tname\tslug\n");
            for (id, name, slug) in rows {
                s.push_str(&format!("{id}\t{name}\t{slug}\n"));
            }
            Ok((s, 0))
        }

        ["environments", "list", proj] => {
            let Ok(project_id) = proj.parse::<Uuid>() else {
                return Ok(("environments list: project_id must be a UUID\n".into(), 1));
            };
            let ok: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM projects WHERE id = $1 AND team_id = $2)",
            )
            .bind(project_id)
            .bind(team_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
            if !ok {
                return Ok(("project not in this team\n".into(), 1));
            }
            let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
                "SELECT id, name, slug FROM environments WHERE project_id = $1 ORDER BY name",
            )
            .bind(project_id)
            .fetch_all(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
            if rows.is_empty() {
                return Ok(("No environments.\n".into(), 0));
            }
            let mut s = String::from("id\tname\tslug\n");
            for (id, name, slug) in rows {
                s.push_str(&format!("{id}\t{name}\t{slug}\n"));
            }
            Ok((s, 0))
        }

        ["applications", "list", proj, env] => {
            let Ok(project_id) = proj.parse::<Uuid>() else {
                return Ok(("applications list: project_id must be a UUID\n".into(), 1));
            };
            let Ok(environment_id) = env.parse::<Uuid>() else {
                return Ok(("applications list: environment_id must be a UUID\n".into(), 1));
            };
            let ok: bool = sqlx::query_scalar(
                r#"SELECT EXISTS(
                    SELECT 1 FROM environments e
                    JOIN projects p ON p.id = e.project_id
                    WHERE e.id = $1 AND e.project_id = $2 AND p.team_id = $3)"#,
            )
            .bind(environment_id)
            .bind(project_id)
            .bind(team_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
            if !ok {
                return Ok(("environment / project not in this team\n".into(), 1));
            }
            let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
                "SELECT id, name, slug FROM applications WHERE environment_id = $1 ORDER BY name",
            )
            .bind(environment_id)
            .fetch_all(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
            if rows.is_empty() {
                return Ok(("No applications.\n".into(), 0));
            }
            let mut s = String::from("id\tname\tslug\n");
            for (id, name, slug) in rows {
                s.push_str(&format!("{id}\t{name}\t{slug}\n"));
            }
            Ok((s, 0))
        }

        ["tokens"] | ["tokens", "list"] => {
            let rows: Vec<(Uuid, String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
                "SELECT id, name, created_at FROM api_tokens WHERE user_id = $1 ORDER BY created_at DESC",
            )
            .bind(p.user_id)
            .fetch_all(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
            if rows.is_empty() {
                return Ok(("No API tokens.\n".into(), 0));
            }
            let mut s = String::from("id\tname\tcreated_at\n");
            for (id, name, created_at) in rows {
                s.push_str(&format!("{id}\t{name}\t{created_at}\n"));
            }
            Ok((s, 0))
        }

        ["servers"] | ["servers", "list"] => {
            let rows: Vec<(Uuid, String, String, i32, String, String)> = sqlx::query_as(
                r#"SELECT id, name, host, ssh_port, ssh_user, status
                   FROM servers WHERE team_id = $1 ORDER BY name"#,
            )
            .bind(team_id)
            .fetch_all(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
            if rows.is_empty() {
                return Ok(("No servers.\n".into(), 0));
            }
            let mut s = String::from("id\tname\thost\tport\tuser\tstatus\n");
            for (id, name, host, port, user, status) in rows {
                s.push_str(&format!("{id}\t{name}\t{host}\t{port}\t{user}\t{status}\n"));
            }
            Ok((s, 0))
        }

        ["deploy", app_str] => {
            p.require_deploy()?;
            let Ok(application_id) = app_str.parse::<Uuid>() else {
                return Ok(("deploy: application_id must be a UUID\n".into(), 1));
            };
            match enqueue_deploy_application_in_team(
                state.clone(),
                p.user_id,
                team_id,
                application_id,
            )
            .await
            {
                Ok(body) => {
                    try_log_team_audit(
                        &state.pool,
                        team_id,
                        p.user_id,
                        "deploy_enqueued",
                        "deploy_job",
                        Some(body.job_id),
                        serde_json::json!({ "application_id": application_id, "source": "web_cli" }),
                        None,
                    )
                    .await;
                    Ok((
                        format!(
                            "queued deploy job {}\nstatus: {:?}\n",
                            body.job_id, body.status
                        ),
                        0,
                    ))
                }
                Err(ApiError::BadRequest(m)) => Ok((format!("{m}\n"), 1)),
                Err(ApiError::Forbidden) => Ok((
                    "forbidden: deploy scope or application access denied\n".into(),
                    1,
                )),
                Err(ApiError::NotFound) => Ok(("application not found in this team\n".into(), 1)),
                Err(e) => Err(e),
            }
        }

        _ => Ok((
            "Unknown command. Type 'help' for allowlisted commands.\n".into(),
            1,
        )),
    }
}
