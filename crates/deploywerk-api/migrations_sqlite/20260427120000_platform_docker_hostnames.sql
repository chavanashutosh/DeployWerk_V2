PRAGMA foreign_keys = OFF;

CREATE TABLE destinations_new (
    id TEXT PRIMARY KEY NOT NULL,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    server_id TEXT REFERENCES servers(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('docker_standalone', 'docker_platform')),
    description TEXT,
    created_at TEXT NOT NULL,
    UNIQUE(team_id, slug),
    CHECK (
        (kind = 'docker_platform' AND server_id IS NULL)
        OR (kind = 'docker_standalone' AND server_id IS NOT NULL)
    )
);

INSERT INTO destinations_new (id, team_id, server_id, name, slug, kind, description, created_at)
SELECT id, team_id, server_id, name, slug, kind, description, created_at FROM destinations;

DROP TABLE destinations;
ALTER TABLE destinations_new RENAME TO destinations;

CREATE INDEX idx_destinations_team ON destinations(team_id);
CREATE INDEX idx_destinations_server ON destinations(server_id);

ALTER TABLE applications ADD COLUMN auto_hostname TEXT NULL;

CREATE UNIQUE INDEX idx_applications_auto_hostname_unique
    ON applications (auto_hostname)
    WHERE auto_hostname IS NOT NULL;

PRAGMA foreign_keys = ON;
