use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(name = "deploywerk")]
#[command(about = "DeployWerk CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// API base URL (overrides config)
    #[arg(long, global = true, env = "DEPLOYWERK_API_URL")]
    base_url: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Authentication
    Auth {
        #[command(subcommand)]
        sub: AuthCommands,
    },
    /// List teams for the current user
    Teams {
        #[command(subcommand)]
        sub: TeamCommands,
    },
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Sign in and save token to config
    Login {
        #[arg(long)]
        email: String,
    },
}

#[derive(Subcommand)]
enum TeamCommands {
    List,
}

#[derive(Serialize, Deserialize, Default, Clone)]
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

#[derive(Deserialize)]
struct AuthResponse {
    token: String,
}

#[derive(Deserialize)]
struct TeamRow {
    id: String,
    name: String,
    slug: String,
    role: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut cfg = load_config()?;
    if let Some(url) = cli.base_url.clone() {
        cfg.base_url = url.trim_end_matches('/').to_string();
    }

    match cli.command {
        Commands::Auth { sub } => match sub {
            AuthCommands::Login { email } => {
                let password = rpassword::prompt_password("Password: ")?;
                let client = reqwest::Client::new();
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
        },
        Commands::Teams { sub } => match sub {
            TeamCommands::List => {
                let token = cfg
                    .token
                    .as_deref()
                    .context("not logged in; run `deploywerk auth login`")?;
                let client = reqwest::Client::new();
                let url = format!("{}/api/v1/teams", cfg.base_url);
                let res = client
                    .get(&url)
                    .header("Authorization", format!("Bearer {}", token))
                    .send()
                    .await
                    .context("teams request failed")?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("teams failed: {}", text);
                }
                let teams: Vec<TeamRow> = res.json().await.context("invalid teams response")?;
                if teams.is_empty() {
                    println!("No teams.");
                } else {
                    for t in teams {
                        println!("{}  {}  ({})  [{}]", t.id, t.name, t.slug, t.role);
                    }
                }
            }
        },
    }

    Ok(())
}
