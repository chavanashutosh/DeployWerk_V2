//! Core domain types and errors shared by API and CLI.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type UserId = Uuid;
pub type OrganizationId = Uuid;
pub type TeamId = Uuid;
pub type ProjectId = Uuid;
pub type EnvironmentId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TeamRole {
    Owner,
    Admin,
    Member,
}

impl TeamRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            TeamRole::Owner => "owner",
            TeamRole::Admin => "admin",
            TeamRole::Member => "member",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "owner" => Some(TeamRole::Owner),
            "admin" => Some(TeamRole::Admin),
            "member" => Some(TeamRole::Member),
            _ => None,
        }
    }

    /// Strongest of two roles (for merging org/team membership).
    pub fn max_rank(a: Self, b: Self) -> Self {
        use TeamRole::*;
        match (a, b) {
            (Owner, _) | (_, Owner) => Owner,
            (Admin, _) | (_, Admin) => Admin,
            _ => Member,
        }
    }
}

/// Per-application access (Authentik SCIM: `deploywerk-app-{uuid}-admin|viewer`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppRole {
    Admin,
    Viewer,
}

impl AppRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppRole::Admin => "admin",
            AppRole::Viewer => "viewer",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(AppRole::Admin),
            "viewer" => Some(AppRole::Viewer),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationMembershipSummary {
    pub application_id: Uuid,
    pub role: AppRole,
}

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("invalid team role")]
    InvalidTeamRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSummary {
    pub id: UserId,
    pub email: String,
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_team_id: Option<TeamId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_organization_id: Option<OrganizationId>,
    /// Client preferences (theme, locale, etc.) from `user_preferences.settings_json`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<serde_json::Value>,
    /// DeployWerk platform operator (not team/org role).
    #[serde(default)]
    pub is_platform_admin: bool,
    /// Organizations where the user is owner or admin (org-wide settings; not app-admin).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub organization_admin_organization_ids: Vec<OrganizationId>,
    /// Explicit app-scoped roles (viewer / admin).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub application_memberships: Vec<ApplicationMembershipSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationSummary {
    pub id: OrganizationId,
    pub name: String,
    pub slug: String,
    pub role: TeamRole,
    #[serde(default)]
    pub mfa_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSummary {
    pub id: TeamId,
    pub organization_id: OrganizationId,
    pub name: String,
    pub slug: String,
    pub role: TeamRole,
    /// User reached this team via org owner/admin membership, not `team_memberships`.
    #[serde(default)]
    pub access_via_organization_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: ProjectId,
    pub team_id: TeamId,
    pub name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSummary {
    pub id: EnvironmentId,
    pub project_id: ProjectId,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
    /// When true, new deploys are rejected until unlocked.
    #[serde(default)]
    pub deploy_locked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deploy_lock_reason: Option<String>,
    /// JSON schedule: `{"utc_start_hour":9,"utc_end_hour":18,"weekdays_only":true}` — omit or null for no restriction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deploy_schedule_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenScopes {
    #[serde(default)]
    pub read: bool,
    #[serde(default)]
    pub write: bool,
    #[serde(default)]
    pub deploy: bool,
}

impl TokenScopes {
    pub fn full() -> Self {
        Self {
            read: true,
            write: true,
            deploy: true,
        }
    }

    pub fn from_list(scopes: &[String]) -> Self {
        let mut s = TokenScopes::default();
        for x in scopes {
            match x.as_str() {
                "read" => s.read = true,
                "write" => s.write = true,
                "deploy" => s.deploy = true,
                _ => {}
            }
        }
        s
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }

    pub fn parse_json(s: &str) -> Self {
        serde_json::from_str(s).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTokenSummary {
    pub id: Uuid,
    pub name: String,
    pub scopes: TokenScopes,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// When non-empty, API requests using this token must come from an IP matching one of these CIDRs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_cidrs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerStatus {
    Pending,
    Ready,
    Error,
}

impl ServerStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ServerStatus::Pending => "pending",
            ServerStatus::Ready => "ready",
            ServerStatus::Error => "error",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(ServerStatus::Pending),
            "ready" => Some(ServerStatus::Ready),
            "error" => Some(ServerStatus::Error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DestinationKind {
    DockerStandalone,
    /// Docker on the same host as the DeployWerk API (operator-enabled).
    DockerPlatform,
}

impl DestinationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DestinationKind::DockerStandalone => "docker_standalone",
            DestinationKind::DockerPlatform => "docker_platform",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "docker_standalone" => Some(DestinationKind::DockerStandalone),
            "docker_platform" => Some(DestinationKind::DockerPlatform),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestinationSummary {
    pub id: Uuid,
    pub team_id: TeamId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub kind: DestinationKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationSummary {
    pub id: Uuid,
    pub environment_id: EnvironmentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub docker_image: String,
    #[serde(default)]
    pub domains: Vec<String>,
    /// Operator-provisioned hostname under DEPLOYWERK_APPS_BASE_DOMAIN (stable; Traefik Host rule).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_repo_url: Option<String>,
    /// Normalized `owner/repo` for GitHub webhook matching.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_repo_full_name: Option<String>,
    #[serde(default)]
    pub auto_deploy_on_push: bool,
    /// Branch filter: exact name, `*` for all branches, or `prefix/*` for path-style prefixes.
    #[serde(default = "default_git_branch_pattern")]
    pub git_branch_pattern: String,
    /// When true, deploy worker clones `git_repo_url` on the target host and runs `docker build` before `docker run`.
    #[serde(default)]
    pub build_image_from_git: bool,
    /// Git ref (branch or tag) for clone when `build_image_from_git` is enabled.
    #[serde(default = "default_git_build_ref")]
    pub git_build_ref: String,
    /// Path to Dockerfile relative to repo root on the remote host.
    #[serde(default = "default_dockerfile_path")]
    pub dockerfile_path: String,
    /// When true, GitHub App `pull_request` hooks may enqueue isolated PR preview deploys.
    #[serde(default)]
    pub pr_preview_enabled: bool,
    pub created_at: DateTime<Utc>,
    /// Image reference last successfully run for this app (standard/rollback deploys).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_deployed_image: Option<String>,
    /// Prior successful image; used for one-step rollback.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_deployed_image: Option<String>,
    /// `standard` | `blue_green` | `canary` | `rolling`
    #[serde(default = "default_deploy_strategy")]
    pub deploy_strategy: String,
    #[serde(default)]
    pub require_deploy_approval: bool,
    /// Optional HTTPS URL; worker POSTs JSON before container replace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_deploy_hook_url: Option<String>,
    /// Optional HTTPS URL; worker POSTs JSON after successful container start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_deploy_hook_url: Option<String>,
}

fn default_deploy_strategy() -> String {
    "standard".into()
}

fn default_git_branch_pattern() -> String {
    "main".into()
}

fn default_git_build_ref() -> String {
    "main".into()
}

fn default_dockerfile_path() -> String {
    "Dockerfile".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeployJobStatus {
    PendingApproval,
    Queued,
    Running,
    Succeeded,
    Failed,
}

impl DeployJobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeployJobStatus::PendingApproval => "pending_approval",
            DeployJobStatus::Queued => "queued",
            DeployJobStatus::Running => "running",
            DeployJobStatus::Succeeded => "succeeded",
            DeployJobStatus::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending_approval" => Some(DeployJobStatus::PendingApproval),
            "queued" => Some(DeployJobStatus::Queued),
            "running" => Some(DeployJobStatus::Running),
            "succeeded" => Some(DeployJobStatus::Succeeded),
            "failed" => Some(DeployJobStatus::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployJobSummary {
    pub id: Uuid,
    pub application_id: Uuid,
    pub status: DeployJobStatus,
    pub log: String,
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
    pub deploy_strategy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_object_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_manifest_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationEnvVarPublic {
    pub key: String,
    /// Secret values are returned as `null` on read; use PATCH to set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    pub is_secret: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeVolumeMount {
    /// Logical name for the mount (used for deterministic host directory naming).
    pub name: String,
    /// Absolute path inside the container, e.g. `/data`.
    pub container_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationDetail {
    #[serde(flatten)]
    pub application: ApplicationSummary,
    pub env_vars: Vec<ApplicationEnvVarPublic>,
    #[serde(default)]
    pub runtime_volumes: Vec<RuntimeVolumeMount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSummary {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub host: String,
    pub ssh_port: i32,
    pub ssh_user: String,
    pub status: ServerStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_validated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_validation_error: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationPublic {
    pub team_name: String,
    pub team_slug: String,
    pub email: String,
    pub role: TeamRole,
    pub expires_at: DateTime<Utc>,
    pub accepted: bool,
    /// True when `expires_at` is in the past (invite cannot be accepted).
    #[serde(default)]
    pub expired: bool,
}
