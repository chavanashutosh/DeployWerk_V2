//! Static permission keys for future custom roles and policy checks (Phase 3 foundation).

#[allow(dead_code)]
pub const PERMISSION_KEYS: &[&str] = &[
    "team.read",
    "team.write",
    "deploy.trigger",
    "secrets.read",
    "secrets.write",
    "audit.read",
];
