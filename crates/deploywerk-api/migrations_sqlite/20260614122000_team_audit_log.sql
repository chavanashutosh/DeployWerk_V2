CREATE TABLE IF NOT EXISTS team_audit_log (
    id TEXT PRIMARY KEY NOT NULL,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    actor_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',
    source_ip TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_team_audit_log_team_time ON team_audit_log(team_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_team_audit_log_team_entity ON team_audit_log(team_id, entity_type, created_at DESC);
