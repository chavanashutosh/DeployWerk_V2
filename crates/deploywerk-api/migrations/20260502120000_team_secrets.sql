-- Team-scoped named secrets (encrypted at rest with SERVER_KEY_ENCRYPTION_KEY).

CREATE TABLE team_secrets (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    value_ciphertext BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    UNIQUE(team_id, name)
);

CREATE INDEX idx_team_secrets_team ON team_secrets(team_id);
