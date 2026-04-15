use std::fs;
use std::io::Write;
use std::path::PathBuf;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Serialize;
use tabwriter::TabWriter;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "deploywerk")]
#[command(about = "DeployWerk CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// API base URL (overrides config)
    #[arg(long, global = true, env = "DEPLOYWERK_API_URL")]
    base_url: Option<String>,

    /// Print list command output as JSON (for scripting)
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    Auth {
        #[command(subcommand)]
        sub: AuthCommands,
    },
    Teams {
        #[command(subcommand)]
        sub: TeamCommands,
    },
    Projects {
        #[command(subcommand)]
        sub: ProjectCommands,
    },
    Environments {
        #[command(subcommand)]
        sub: EnvironmentCommands,
    },
    Tokens {
        #[command(subcommand)]
        sub: TokenCommands,
    },
    Deploy {
        #[command(subcommand)]
        sub: DeployCommands,
    },
    Servers {
        #[command(subcommand)]
        sub: ServerCommands,
    },
    Destinations {
        #[command(subcommand)]
        sub: DestinationCommands,
    },
    Applications {
        #[command(subcommand)]
        sub: ApplicationCommands,
    },
    Organizations {
        #[command(subcommand)]
        sub: OrgCommands,
    },
    Invitations {
        #[command(subcommand)]
        sub: InvCommands,
    },
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Save a session JWT from password login (stored in the CLI config file).
    Login {
        #[arg(long)]
        email: String,
    },
    /// Show configured API URL and whether a token is stored (token is never printed).
    Status,
    /// Remove the stored token.
    Logout,
}

#[derive(Subcommand)]
enum TeamCommands {
    List,
}

#[derive(Subcommand)]
enum ProjectCommands {
    List {
        #[arg(long)]
        team: Uuid,
    },
}

#[derive(Subcommand)]
enum EnvironmentCommands {
    List {
        #[arg(long)]
        team: Uuid,
        #[arg(long)]
        project: Uuid,
    },
}

#[derive(Subcommand)]
enum TokenCommands {
    List,
    Create {
        #[arg(long)]
        name: String,
        /// Comma-separated: read,write,deploy
        #[arg(long, default_value = "read")]
        scopes: String,
    },
    Revoke {
        #[arg(long)]
        id: Uuid,
    },
}

#[derive(Subcommand)]
enum DeployCommands {
    Trigger {
        application_id: Uuid,
    },
}

#[derive(Subcommand)]
enum ServerCommands {
    List {
        #[arg(long)]
        team: Uuid,
    },
    Validate {
        #[arg(long)]
        team: Uuid,
        #[arg(long)]
        server: Uuid,
    },
    Create {
        #[arg(long)]
        team: Uuid,
        #[arg(long)]
        name: String,
        #[arg(long)]
        host: String,
        #[arg(long, default_value_t = 22)]
        port: i32,
        #[arg(long)]
        user: String,
        #[arg(long)]
        key_file: PathBuf,
    },
    Update {
        #[arg(long)]
        team: Uuid,
        #[arg(long)]
        server: Uuid,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        port: Option<i32>,
        #[arg(long)]
        user: Option<String>,
        #[arg(long)]
        key_file: Option<PathBuf>,
    },
    Delete {
        #[arg(long)]
        team: Uuid,
        #[arg(long)]
        server: Uuid,
    },
}

#[derive(Subcommand)]
enum DestinationCommands {
    List {
        #[arg(long)]
        team: Uuid,
    },
    Create {
        #[arg(long)]
        team: Uuid,
        /// Required for `docker_standalone`; omit for `docker_platform`.
        #[arg(long)]
        server: Option<Uuid>,
        #[arg(long)]
        name: String,
        #[arg(long)]
        slug: String,
        #[arg(long, default_value = "docker_standalone")]
        kind: String,
        #[arg(long)]
        description: Option<String>,
    },
    Delete {
        #[arg(long)]
        team: Uuid,
        #[arg(long)]
        destination: Uuid,
    },
}

#[derive(Subcommand)]
enum ApplicationCommands {
    List {
        #[arg(long)]
        team: Uuid,
        #[arg(long)]
        project: Uuid,
        #[arg(long)]
        environment: Uuid,
    },
    Create {
        #[arg(long)]
        team: Uuid,
        #[arg(long)]
        project: Uuid,
        #[arg(long)]
        environment: Uuid,
        #[arg(long)]
        name: String,
        #[arg(long)]
        docker_image: String,
        #[arg(long)]
        slug: Option<String>,
        #[arg(long)]
        destination: Option<Uuid>,
        #[arg(long)]
        git_repo_url: Option<String>,
        #[arg(long)]
        git_repo_full_name: Option<String>,
        #[arg(long, default_value_t = false)]
        auto_deploy_on_push: bool,
        #[arg(long)]
        git_branch_pattern: Option<String>,
        #[arg(long, default_value_t = false)]
        pr_preview_enabled: bool,
    },
    Patch {
        #[arg(long)]
        team: Uuid,
        #[arg(long)]
        project: Uuid,
        #[arg(long)]
        environment: Uuid,
        #[arg(long)]
        application: Uuid,
        /// JSON body matching the API PATCH schema (partial fields).
        #[arg(long)]
        file: PathBuf,
    },
    Deploy {
        #[arg(long)]
        team: Uuid,
        #[arg(long)]
        project: Uuid,
        #[arg(long)]
        environment: Uuid,
        #[arg(long)]
        application: Uuid,
    },
    Rollback {
        #[arg(long)]
        team: Uuid,
        #[arg(long)]
        project: Uuid,
        #[arg(long)]
        environment: Uuid,
        #[arg(long)]
        application: Uuid,
    },
}

#[derive(Subcommand)]
enum OrgCommands {
    List,
}

#[derive(Subcommand)]
enum InvCommands {
    /// Accept a pending invitation (JWT session required).
    Accept {
        #[arg(long)]
        invitation_token: String,
    },
}

#[derive(Serialize, Deserialize, Clone)]
struct ConfigFile {
    base_url: String,
    token: Option<String>,
}

fn config_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("dev", "deploywerk", "deploywerk-cli")
        .context("could not resolve config directory")?;
    let dir = dirs.config_dir();
    fs::create_dir_all(dir)?;
    Ok(dir.join("config.json"))
}

fn default_config() -> ConfigFile {
    ConfigFile {
        base_url: "http://127.0.0.1:8080".into(),
        token: None,
    }
}

fn load_config() -> Result<ConfigFile> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(default_config());
    }
    let s = fs::read_to_string(&path)?;
    match serde_json::from_str::<ConfigFile>(&s) {
        Ok(mut c) => {
            if c.base_url.trim().is_empty() {
                c.base_url = default_config().base_url;
            }
            Ok(c)
        }
        Err(_) => Ok(default_config()),
    }
}

fn save_config(cfg: &ConfigFile) -> Result<()> {
    let path = config_path()?;
    fs::write(path, serde_json::to_string_pretty(cfg)?)?;
    Ok(())
}

fn print_json_pretty<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

#[derive(Deserialize)]
struct AuthResponse {
    token: String,
}

#[derive(Deserialize, Serialize)]
struct TeamRow {
    id: String,
    name: String,
    slug: String,
    role: String,
}

#[derive(Deserialize, Serialize)]
struct ProjectRow {
    id: String,
    name: String,
    slug: String,
}

#[derive(Deserialize, Serialize)]
struct EnvironmentRow {
    id: String,
    name: String,
    slug: String,
}

#[derive(Deserialize, Serialize)]
struct TokenRow {
    id: String,
    name: String,
    scopes: serde_json::Value,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let json_out = cli.json;
    let mut cfg = load_config()?;
    if let Some(url) = cli.base_url.clone() {
        cfg.base_url = url.trim_end_matches('/').to_string();
    }

    let token = || {
        cfg.token
            .as_deref()
            .context("not logged in; run `deploywerk auth login` or set token in config")
    };

    let client = || reqwest::Client::new();

    match cli.command {
        Commands::Auth { sub } => match sub {
            AuthCommands::Login { email } => {
                let password = rpassword::prompt_password("Password: ")?;
                let client = client();
                let url = format!("{}/api/v1/auth/login", cfg.base_url);
                let res = client
                    .post(&url)
                    .json(&serde_json::json!({ "email": email, "password": password }))
                    .send()
                    .await
                    .context("login request failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("login failed: {}", text);
                }
                let body: AuthResponse = res.json().await.context("invalid login response")?;
                cfg.token = Some(body.token);
                save_config(&cfg)?;
                println!("Logged in. Token saved to {:?}.", config_path()?);
            }
            AuthCommands::Status => {
                let path = config_path()?;
                let has_token = cfg
                    .token
                    .as_ref()
                    .map(|t| !t.trim().is_empty())
                    .unwrap_or(false);
                if json_out {
                    print_json_pretty(&serde_json::json!({
                        "base_url": cfg.base_url,
                        "config_path": path,
                        "logged_in": has_token,
                    }))?;
                } else {
                    println!("API URL:   {}", cfg.base_url);
                    println!("Config:    {:?}", path);
                    println!(
                        "Logged in: {}",
                        if has_token {
                            "yes (JWT stored; not shown)"
                        } else {
                            "no — run `deploywerk auth login`"
                        }
                    );
                }
            }
            AuthCommands::Logout => {
                cfg.token = None;
                save_config(&cfg)?;
                println!("Removed stored token from {:?}.", config_path()?);
            }
        },
        Commands::Teams { sub } => match sub {
            TeamCommands::List => {
                let t = token()?;
                let client = client();
                let url = format!("{}/api/v1/teams", cfg.base_url);
                let res = client
                    .get(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .send()
                    .await
                    .context("teams request failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("teams failed: {}", text);
                }
                let teams: Vec<TeamRow> = res.json().await.context("invalid teams response")?;
                if json_out {
                    print_json_pretty(&teams)?;
                } else if teams.is_empty() {
                    println!("No teams.");
                } else {
                    let mut tw = TabWriter::new(Vec::new());
                    writeln!(tw, "id\tname\tslug\trole")?;
                    for row in &teams {
                        writeln!(tw, "{}\t{}\t{}\t{}", row.id, row.name, row.slug, row.role)?;
                    }
                    tw.flush()?;
                    print!("{}", String::from_utf8(tw.into_inner()?)?);
                }
            }
        },
        Commands::Projects { sub } => match sub {
            ProjectCommands::List { team } => {
                let t = token()?;
                let client = client();
                let url = format!("{}/api/v1/teams/{}/projects", cfg.base_url, team);
                let res = client
                    .get(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .send()
                    .await
                    .context("projects request failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("projects failed: {}", text);
                }
                let projects: Vec<ProjectRow> = res.json().await.context("invalid response")?;
                if json_out {
                    print_json_pretty(&projects)?;
                } else if projects.is_empty() {
                    println!("No projects.");
                } else {
                    let mut tw = TabWriter::new(Vec::new());
                    writeln!(tw, "id\tname\tslug")?;
                    for p in &projects {
                        writeln!(tw, "{}\t{}\t{}", p.id, p.name, p.slug)?;
                    }
                    tw.flush()?;
                    print!("{}", String::from_utf8(tw.into_inner()?)?);
                }
            }
        },
        Commands::Environments { sub } => match sub {
            EnvironmentCommands::List { team, project } => {
                let t = token()?;
                let client = client();
                let url = format!(
                    "{}/api/v1/teams/{}/projects/{}/environments",
                    cfg.base_url, team, project
                );
                let res = client
                    .get(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .send()
                    .await
                    .context("environments request failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("environments failed: {}", text);
                }
                let envs: Vec<EnvironmentRow> = res.json().await.context("invalid response")?;
                if json_out {
                    print_json_pretty(&envs)?;
                } else if envs.is_empty() {
                    println!("No environments.");
                } else {
                    let mut tw = TabWriter::new(Vec::new());
                    writeln!(tw, "id\tname\tslug")?;
                    for e in &envs {
                        writeln!(tw, "{}\t{}\t{}", e.id, e.name, e.slug)?;
                    }
                    tw.flush()?;
                    print!("{}", String::from_utf8(tw.into_inner()?)?);
                }
            }
        },
        Commands::Tokens { sub } => match sub {
            TokenCommands::List => {
                let t = token()?;
                let client = client();
                let url = format!("{}/api/v1/tokens", cfg.base_url);
                let res = client
                    .get(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .send()
                    .await
                    .context("tokens request failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("tokens failed: {}", text);
                }
                let rows: Vec<TokenRow> = res.json().await.context("invalid response")?;
                if json_out {
                    print_json_pretty(&rows)?;
                } else if rows.is_empty() {
                    println!("No API tokens.");
                } else {
                    let mut tw = TabWriter::new(Vec::new());
                    writeln!(tw, "id\tname\tscopes")?;
                    for r in &rows {
                        writeln!(tw, "{}\t{}\t{}", r.id, r.name, r.scopes)?;
                    }
                    tw.flush()?;
                    print!("{}", String::from_utf8(tw.into_inner()?)?);
                }
            }
            TokenCommands::Create { name, scopes } => {
                let t = token()?;
                let scope_list: Vec<String> = scopes
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let client = client();
                let url = format!("{}/api/v1/tokens", cfg.base_url);
                let res = client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .json(&serde_json::json!({ "name": name, "scopes": scope_list }))
                    .send()
                    .await
                    .context("create token failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("create token failed: {}", text);
                }
                let body: serde_json::Value = res.json().await.context("invalid response")?;
                println!("{}", serde_json::to_string_pretty(&body)?);
            }
            TokenCommands::Revoke { id } => {
                let t = token()?;
                let client = client();
                let url = format!("{}/api/v1/tokens/{}", cfg.base_url, id);
                let res = client
                    .delete(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .send()
                    .await
                    .context("revoke failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("revoke failed: {}", text);
                }
                println!("Revoked.");
            }
        },
        Commands::Servers { sub } => match sub {
            ServerCommands::List { team } => {
                let t = token()?;
                let client = client();
                let url = format!("{}/api/v1/teams/{}/servers", cfg.base_url, team);
                let res = client
                    .get(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .send()
                    .await
                    .context("servers request failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("servers failed: {}", text);
                }
                let rows: Vec<serde_json::Value> = res.json().await.context("invalid response")?;
                if json_out {
                    print_json_pretty(&rows)?;
                } else if rows.is_empty() {
                    println!("No servers.");
                } else {
                    println!("{}", serde_json::to_string_pretty(&rows)?);
                }
            }
            ServerCommands::Validate { team, server } => {
                let t = token()?;
                let client = client();
                let url = format!(
                    "{}/api/v1/teams/{}/servers/{}/validate",
                    cfg.base_url, team, server
                );
                let res = client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .send()
                    .await
                    .context("validate request failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("validate failed: {}", text);
                }
                let body: serde_json::Value = res.json().await.context("invalid response")?;
                println!("{}", serde_json::to_string_pretty(&body)?);
            }
            ServerCommands::Create {
                team,
                name,
                host,
                port,
                user,
                key_file,
            } => {
                let t = token()?;
                let pem = fs::read_to_string(&key_file)
                    .with_context(|| format!("read key file {:?}", key_file))?;
                let client = client();
                let url = format!("{}/api/v1/teams/{}/servers", cfg.base_url, team);
                let res = client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .json(&serde_json::json!({
                        "name": name,
                        "host": host,
                        "ssh_port": port,
                        "ssh_user": user,
                        "ssh_private_key_pem": pem,
                    }))
                    .send()
                    .await
                    .context("create server failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("create server failed: {}", text);
                }
                let body: serde_json::Value = res.json().await.context("invalid response")?;
                println!("{}", serde_json::to_string_pretty(&body)?);
            }
            ServerCommands::Update {
                team,
                server,
                name,
                host,
                port,
                user,
                key_file,
            } => {
                let t = token()?;
                let mut body = serde_json::Map::new();
                if let Some(n) = name {
                    body.insert("name".into(), serde_json::Value::String(n));
                }
                if let Some(h) = host {
                    body.insert("host".into(), serde_json::Value::String(h));
                }
                if let Some(p) = port {
                    body.insert(
                        "ssh_port".into(),
                        serde_json::Value::Number(serde_json::Number::from(p)),
                    );
                }
                if let Some(u) = user {
                    body.insert("ssh_user".into(), serde_json::Value::String(u));
                }
                if let Some(ref path) = key_file {
                    let pem = fs::read_to_string(path)
                        .with_context(|| format!("read key file {:?}", path))?;
                    body.insert(
                        "ssh_private_key_pem".into(),
                        serde_json::Value::String(pem),
                    );
                }
                if body.is_empty() {
                    anyhow::bail!("provide at least one of --name --host --port --user --key-file");
                }
                let client = client();
                let url = format!(
                    "{}/api/v1/teams/{}/servers/{}",
                    cfg.base_url, team, server
                );
                let res = client
                    .patch(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .json(&serde_json::Value::Object(body))
                    .send()
                    .await
                    .context("update server failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("update server failed: {}", text);
                }
                let body: serde_json::Value = res.json().await.context("invalid response")?;
                println!("{}", serde_json::to_string_pretty(&body)?);
            }
            ServerCommands::Delete { team, server } => {
                let t = token()?;
                let client = client();
                let url = format!(
                    "{}/api/v1/teams/{}/servers/{}",
                    cfg.base_url, team, server
                );
                let res = client
                    .delete(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .send()
                    .await
                    .context("delete server failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("delete server failed: {}", text);
                }
                println!("Deleted.");
            }
        },
        Commands::Destinations { sub } => match sub {
            DestinationCommands::List { team } => {
                let t = token()?;
                let client = client();
                let url = format!("{}/api/v1/teams/{}/destinations", cfg.base_url, team);
                let res = client
                    .get(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .send()
                    .await
                    .context("destinations request failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("destinations failed: {}", text);
                }
                let rows: Vec<serde_json::Value> = res.json().await.context("invalid response")?;
                println!("{}", serde_json::to_string_pretty(&rows)?);
            }
            DestinationCommands::Create {
                team,
                server,
                name,
                slug,
                kind,
                description,
            } => {
                let t = token()?;
                let client = client();
                let url = format!("{}/api/v1/teams/{}/destinations", cfg.base_url, team);
                let kind = kind.trim().to_lowercase();
                if kind == "docker_standalone" && server.is_none() {
                    anyhow::bail!("--server is required for docker_standalone");
                }
                let mut body = serde_json::json!({
                    "name": name,
                    "slug": slug,
                    "kind": kind,
                    "description": description,
                });
                if let Some(sid) = server {
                    body["server_id"] = serde_json::json!(sid.to_string());
                }
                let res = client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .json(&body)
                    .send()
                    .await
                    .context("create destination failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("create destination failed: {}", text);
                }
                let body: serde_json::Value = res.json().await.context("invalid response")?;
                println!("{}", serde_json::to_string_pretty(&body)?);
            }
            DestinationCommands::Delete { team, destination } => {
                let t = token()?;
                let client = client();
                let url = format!(
                    "{}/api/v1/teams/{}/destinations/{}",
                    cfg.base_url, team, destination
                );
                let res = client
                    .delete(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .send()
                    .await
                    .context("delete destination failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("delete destination failed: {}", text);
                }
                println!("Deleted.");
            }
        },
        Commands::Applications { sub } => match sub {
            ApplicationCommands::List {
                team,
                project,
                environment,
            } => {
                let t = token()?;
                let client = client();
                let url = format!(
                    "{}/api/v1/teams/{}/projects/{}/environments/{}/applications",
                    cfg.base_url, team, project, environment
                );
                let res = client
                    .get(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .send()
                    .await
                    .context("applications request failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("applications failed: {}", text);
                }
                let rows: Vec<serde_json::Value> = res.json().await.context("invalid response")?;
                println!("{}", serde_json::to_string_pretty(&rows)?);
            }
            ApplicationCommands::Create {
                team,
                project,
                environment,
                name,
                docker_image,
                slug,
                destination,
                git_repo_url,
                git_repo_full_name,
                auto_deploy_on_push,
                git_branch_pattern,
                pr_preview_enabled,
            } => {
                let t = token()?;
                let client = client();
                let url = format!(
                    "{}/api/v1/teams/{}/projects/{}/environments/{}/applications",
                    cfg.base_url, team, project, environment
                );
                let mut body = serde_json::json!({
                    "name": name,
                    "docker_image": docker_image,
                });
                if let Some(s) = slug {
                    body["slug"] = serde_json::json!(s);
                }
                if let Some(d) = destination {
                    body["destination_id"] = serde_json::json!(d.to_string());
                }
                if let Some(ref u) = git_repo_url {
                    body["git_repo_url"] = serde_json::json!(u);
                }
                if let Some(ref n) = git_repo_full_name {
                    body["git_repo_full_name"] = serde_json::json!(n);
                }
                if auto_deploy_on_push {
                    body["auto_deploy_on_push"] = serde_json::json!(true);
                }
                if let Some(ref p) = git_branch_pattern {
                    body["git_branch_pattern"] = serde_json::json!(p);
                }
                if pr_preview_enabled {
                    body["pr_preview_enabled"] = serde_json::json!(true);
                }
                let res = client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .json(&body)
                    .send()
                    .await
                    .context("create application failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("create application failed: {}", text);
                }
                let body: serde_json::Value = res.json().await.context("invalid response")?;
                println!("{}", serde_json::to_string_pretty(&body)?);
            }
            ApplicationCommands::Patch {
                team,
                project,
                environment,
                application,
                file,
            } => {
                let t = token()?;
                let raw = fs::read_to_string(&file)
                    .with_context(|| format!("read patch JSON {:?}", file))?;
                let json: serde_json::Value =
                    serde_json::from_str(&raw).context("invalid JSON in --file")?;
                let client = client();
                let url = format!(
                    "{}/api/v1/teams/{}/projects/{}/environments/{}/applications/{}",
                    cfg.base_url, team, project, environment, application
                );
                let res = client
                    .patch(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .json(&json)
                    .send()
                    .await
                    .context("patch application failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("patch application failed: {}", text);
                }
                let body: serde_json::Value = res.json().await.context("invalid response")?;
                println!("{}", serde_json::to_string_pretty(&body)?);
            }
            ApplicationCommands::Deploy {
                team,
                project,
                environment,
                application,
            } => {
                let t = token()?;
                let client = client();
                let url = format!(
                    "{}/api/v1/teams/{}/projects/{}/environments/{}/applications/{}/deploy",
                    cfg.base_url, team, project, environment, application
                );
                let res = client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .send()
                    .await
                    .context("deploy request failed")?;
                if res.status() == reqwest::StatusCode::ACCEPTED {
                    let body: serde_json::Value = res.json().await.unwrap_or_default();
                    println!("{}", serde_json::to_string_pretty(&body)?);
                } else if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("deploy failed: {}", text);
                } else {
                    let body: serde_json::Value = res.json().await.unwrap_or_default();
                    println!("{}", serde_json::to_string_pretty(&body)?);
                }
            }
            ApplicationCommands::Rollback {
                team,
                project,
                environment,
                application,
            } => {
                let t = token()?;
                let client = client();
                let url = format!(
                    "{}/api/v1/teams/{}/projects/{}/environments/{}/applications/{}/rollback",
                    cfg.base_url, team, project, environment, application
                );
                let res = client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .send()
                    .await
                    .context("rollback request failed")?;
                if res.status() == reqwest::StatusCode::ACCEPTED {
                    let body: serde_json::Value = res.json().await.unwrap_or_default();
                    println!("{}", serde_json::to_string_pretty(&body)?);
                } else if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("rollback failed: {}", text);
                } else {
                    let body: serde_json::Value = res.json().await.unwrap_or_default();
                    println!("{}", serde_json::to_string_pretty(&body)?);
                }
            }
        },
        Commands::Organizations { sub } => match sub {
            OrgCommands::List => {
                let t = token()?;
                let client = client();
                let url = format!("{}/api/v1/organizations", cfg.base_url);
                let res = client
                    .get(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .send()
                    .await
                    .context("organizations request failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("organizations failed: {}", text);
                }
                let rows: Vec<serde_json::Value> = res.json().await.context("invalid response")?;
                println!("{}", serde_json::to_string_pretty(&rows)?);
            }
        },
        Commands::Invitations { sub } => match sub {
            InvCommands::Accept { invitation_token } => {
                let t = token()?;
                let client = client();
                let url = format!(
                    "{}/api/v1/invitations/{}/accept",
                    cfg.base_url, invitation_token
                );
                let res = client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .send()
                    .await
                    .context("accept invitation failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("accept invitation failed: {}", text);
                }
                let body: serde_json::Value = res.json().await.unwrap_or_default();
                println!("{}", serde_json::to_string_pretty(&body)?);
            }
        },
        Commands::Deploy { sub } => match sub {
            DeployCommands::Trigger { application_id } => {
                let t = token()?;
                let client = client();
                let url = format!(
                    "{}/api/v1/applications/{}/deploy",
                    cfg.base_url, application_id
                );
                let res = client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", t))
                    .send()
                    .await
                    .context("deploy request failed")?;
                if res.status() == reqwest::StatusCode::ACCEPTED {
                    let body: serde_json::Value = res.json().await.unwrap_or_default();
                    println!("{}", serde_json::to_string_pretty(&body)?);
                } else if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("deploy failed: {}", text);
                } else {
                    let body: serde_json::Value = res.json().await.unwrap_or_default();
                    println!("{}", serde_json::to_string_pretty(&body)?);
                }
            }
        },
    }

    Ok(())
}
