ALTER TABLE team_secrets ADD COLUMN latest_version INTEGER NOT NULL DEFAULT 1;

CREATE TABLE IF NOT EXISTS team_secret_versions (
    id TEXT PRIMARY KEY NOT NULL,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    version INTEGER NOT NULL,
    value_ciphertext BLOB NOT NULL,
    created_at TEXT NOT NULL,
    created_by_user_id TEXT NULL REFERENCES users(id) ON DELETE SET NULL,
    UNIQUE(team_id, name, version)
);

CREATE INDEX IF NOT EXISTS idx_team_secret_versions_team_name ON team_secret_versions(team_id, name, version DESC);

INSERT INTO team_secret_versions (id, team_id, name, version, value_ciphertext, created_at, created_by_user_id)
SELECT
    (lower(hex(randomblob(4))) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 3) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 3) || '-' || hex(randomblob(6))),
    s.team_id, s.name, 1, s.value_ciphertext, s.created_at, NULL
FROM team_secrets s
WHERE NOT EXISTS (
    SELECT 1 FROM team_secret_versions v
    WHERE v.team_id = s.team_id AND v.name = s.name
);
