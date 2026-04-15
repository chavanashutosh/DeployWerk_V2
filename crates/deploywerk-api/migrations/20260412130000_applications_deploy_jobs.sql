-- Applications under environments + async deploy job records (P3).

CREATE TABLE applications (
    id UUID PRIMARY KEY NOT NULL,
    environment_id UUID NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    destination_id UUID REFERENCES destinations(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    docker_image TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    UNIQUE(environment_id, slug)
);

CREATE INDEX idx_applications_environment ON applications(environment_id);
CREATE INDEX idx_applications_destination ON applications(destination_id);

CREATE TABLE deploy_jobs (
    id UUID PRIMARY KEY NOT NULL,
    application_id UUID NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed')),
    log TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL,
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ
);

CREATE INDEX idx_deploy_jobs_application ON deploy_jobs(application_id);
