ALTER TABLE environments ADD COLUMN deploy_locked INTEGER NOT NULL DEFAULT 0 CHECK (deploy_locked IN (0, 1));
ALTER TABLE environments ADD COLUMN deploy_lock_reason TEXT NULL;
ALTER TABLE environments ADD COLUMN deploy_schedule_json TEXT NULL;

ALTER TABLE applications ADD COLUMN deploy_strategy TEXT NOT NULL DEFAULT 'standard';
ALTER TABLE applications ADD COLUMN require_deploy_approval INTEGER NOT NULL DEFAULT 0 CHECK (require_deploy_approval IN (0, 1));

PRAGMA foreign_keys = OFF;

CREATE TABLE applications_new (
    id TEXT PRIMARY KEY NOT NULL,
    environment_id TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    destination_id TEXT REFERENCES destinations(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    docker_image TEXT NOT NULL,
    created_at TEXT NOT NULL,
    domains TEXT NOT NULL DEFAULT '[]',
    git_repo_url TEXT,
    runtime_volumes_json TEXT NOT NULL DEFAULT '[]',
    git_repo_full_name TEXT,
    auto_deploy_on_push INTEGER NOT NULL DEFAULT 0 CHECK (auto_deploy_on_push IN (0, 1)),
    git_branch_pattern TEXT NOT NULL DEFAULT 'main',
    build_image_from_git INTEGER NOT NULL DEFAULT 0 CHECK (build_image_from_git IN (0, 1)),
    git_build_ref TEXT NOT NULL DEFAULT 'main',
    dockerfile_path TEXT NOT NULL DEFAULT 'Dockerfile',
    pr_preview_enabled INTEGER NOT NULL DEFAULT 0 CHECK (pr_preview_enabled IN (0, 1)),
    auto_hostname TEXT,
    last_deployed_image TEXT,
    previous_deployed_image TEXT,
    deploy_strategy TEXT NOT NULL DEFAULT 'standard' CHECK (deploy_strategy IN ('standard', 'blue_green', 'canary', 'rolling')),
    require_deploy_approval INTEGER NOT NULL DEFAULT 0 CHECK (require_deploy_approval IN (0, 1)),
    UNIQUE(environment_id, slug)
);

INSERT INTO applications_new SELECT
    id, environment_id, destination_id, name, slug, docker_image, created_at,
    domains, git_repo_url, runtime_volumes_json, git_repo_full_name, auto_deploy_on_push,
    git_branch_pattern, build_image_from_git, git_build_ref, dockerfile_path, pr_preview_enabled,
    auto_hostname, last_deployed_image, previous_deployed_image,
    deploy_strategy, require_deploy_approval
FROM applications;

DROP TABLE applications;
ALTER TABLE applications_new RENAME TO applications;

CREATE INDEX idx_applications_environment ON applications(environment_id);
CREATE INDEX idx_applications_destination ON applications(destination_id);
CREATE INDEX IF NOT EXISTS idx_applications_git_repo ON applications (git_repo_full_name)
    WHERE git_repo_full_name IS NOT NULL AND auto_deploy_on_push = 1;
CREATE INDEX IF NOT EXISTS idx_applications_environment_created ON applications(environment_id, created_at DESC);
CREATE UNIQUE INDEX idx_applications_auto_hostname_unique
    ON applications (auto_hostname)
    WHERE auto_hostname IS NOT NULL;

CREATE TABLE deploy_jobs_new (
    id TEXT PRIMARY KEY NOT NULL,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('pending_approval', 'queued', 'running', 'succeeded', 'failed')),
    log TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    started_at TEXT,
    finished_at TEXT,
    log_object_key TEXT,
    artifact_manifest_key TEXT,
    git_ref TEXT,
    git_sha TEXT,
    job_kind TEXT NOT NULL DEFAULT 'standard' CHECK (job_kind IN ('standard', 'pr_preview', 'pr_preview_destroy', 'rollback')),
    pr_number INTEGER NULL,
    git_base_sha TEXT NULL,
    deploy_strategy TEXT NOT NULL DEFAULT 'standard',
    approved_at TEXT NULL,
    approved_by_user_id TEXT NULL REFERENCES users(id) ON DELETE SET NULL
);

INSERT INTO deploy_jobs_new (
    id, application_id, status, log, created_at, started_at, finished_at,
    log_object_key, artifact_manifest_key, git_ref, git_sha, job_kind, pr_number, git_base_sha,
    deploy_strategy, approved_at, approved_by_user_id
)
SELECT
    id, application_id, status, log, created_at, started_at, finished_at,
    log_object_key, artifact_manifest_key, git_ref, git_sha, job_kind, pr_number, git_base_sha,
    'standard', NULL, NULL
FROM deploy_jobs;

DROP TABLE deploy_jobs;
ALTER TABLE deploy_jobs_new RENAME TO deploy_jobs;

CREATE INDEX idx_deploy_jobs_application ON deploy_jobs(application_id);
CREATE INDEX IF NOT EXISTS idx_deploy_jobs_queued_created_at
  ON deploy_jobs(created_at ASC)
  WHERE status = 'queued';
CREATE INDEX IF NOT EXISTS idx_deploy_jobs_application_created_at
  ON deploy_jobs (application_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_deploy_jobs_created_at ON deploy_jobs(created_at DESC);

PRAGMA foreign_keys = ON;
