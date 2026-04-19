CREATE TABLE applications (
    id TEXT PRIMARY KEY NOT NULL,
    environment_id TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    destination_id TEXT REFERENCES destinations(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    docker_image TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(environment_id, slug)
);

CREATE INDEX idx_applications_environment ON applications(environment_id);
CREATE INDEX idx_applications_destination ON applications(destination_id);

CREATE TABLE deploy_jobs (
    id TEXT PRIMARY KEY NOT NULL,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed')),
    log TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    started_at TEXT,
    finished_at TEXT
);

CREATE INDEX idx_deploy_jobs_application ON deploy_jobs(application_id);
