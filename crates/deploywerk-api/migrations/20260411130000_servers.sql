-- Team-scoped SSH servers (encrypted private keys at rest).

CREATE TABLE servers (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    ssh_port INT NOT NULL DEFAULT 22 CHECK (ssh_port > 0 AND ssh_port <= 65535),
    ssh_user TEXT NOT NULL,
    ssh_private_key_ciphertext BYTEA NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'ready', 'error')),
    last_validated_at TIMESTAMPTZ,
    last_validation_error TEXT,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_servers_team ON servers(team_id);
