ALTER TABLE applications ADD COLUMN last_deployed_image TEXT NULL;
ALTER TABLE applications ADD COLUMN previous_deployed_image TEXT NULL;

PRAGMA foreign_keys = OFF;

CREATE TABLE deploy_jobs_new (
    id TEXT PRIMARY KEY NOT NULL,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed')),
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
    git_base_sha TEXT NULL
);

INSERT INTO deploy_jobs_new SELECT id, application_id, status, log, created_at, started_at, finished_at,
    log_object_key, artifact_manifest_key, git_ref, git_sha, job_kind, pr_number, git_base_sha FROM deploy_jobs;

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
