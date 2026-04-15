-- Team-level audit log for critical mutations (P0).

CREATE TABLE IF NOT EXISTS team_audit_log (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    actor_user_id UUID NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id UUID NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    source_ip TEXT,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_team_audit_log_team_time ON team_audit_log(team_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_team_audit_log_team_entity ON team_audit_log(team_id, entity_type, created_at DESC);

