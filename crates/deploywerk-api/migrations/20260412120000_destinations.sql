-- Docker deployment targets on a team server (P2).

CREATE TABLE destinations (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('docker_standalone')),
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    UNIQUE(team_id, slug)
);

CREATE INDEX idx_destinations_team ON destinations(team_id);
CREATE INDEX idx_destinations_server ON destinations(server_id);
