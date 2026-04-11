//! Core domain types and errors shared by API and CLI.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type UserId = Uuid;
pub type TeamId = Uuid;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSummary {
    pub id: TeamId,
    pub name: String,
    pub slug: String,
    pub role: TeamRole,
}
