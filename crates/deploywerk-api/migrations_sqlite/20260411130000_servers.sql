CREATE TABLE servers (
    id TEXT PRIMARY KEY NOT NULL,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    ssh_port INTEGER NOT NULL DEFAULT 22 CHECK (ssh_port > 0 AND ssh_port <= 65535),
    ssh_user TEXT NOT NULL,
    ssh_private_key_ciphertext BLOB NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'ready', 'error')),
    last_validated_at TEXT,
    last_validation_error TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_servers_team ON servers(team_id);
