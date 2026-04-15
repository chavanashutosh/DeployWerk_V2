-- Secret versioning (P0): keep history for team secrets.

ALTER TABLE team_secrets
    ADD COLUMN IF NOT EXISTS latest_version INT NOT NULL DEFAULT 1;

CREATE TABLE IF NOT EXISTS team_secret_versions (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    version INT NOT NULL,
    value_ciphertext BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    created_by_user_id UUID NULL REFERENCES users(id) ON DELETE SET NULL,
    UNIQUE(team_id, name, version)
);

CREATE INDEX IF NOT EXISTS idx_team_secret_versions_team_name ON team_secret_versions(team_id, name, version DESC);

-- Backfill: existing secrets become version 1 if no versions exist yet.
INSERT INTO team_secret_versions (id, team_id, name, version, value_ciphertext, created_at, created_by_user_id)
SELECT gen_random_uuid(), s.team_id, s.name, 1, s.value_ciphertext, s.created_at, NULL
FROM team_secrets s
WHERE NOT EXISTS (
    SELECT 1 FROM team_secret_versions v
    WHERE v.team_id = s.team_id AND v.name = s.name
);

