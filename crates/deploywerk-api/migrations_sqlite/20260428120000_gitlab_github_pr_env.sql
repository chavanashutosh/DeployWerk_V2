ALTER TABLE teams ADD COLUMN gitlab_webhook_secret TEXT;

ALTER TABLE applications ADD COLUMN pr_preview_enabled INTEGER NOT NULL DEFAULT 0 CHECK (pr_preview_enabled IN (0, 1));

ALTER TABLE deploy_jobs ADD COLUMN job_kind TEXT NOT NULL DEFAULT 'standard';
ALTER TABLE deploy_jobs ADD COLUMN pr_number INTEGER NULL;

CREATE TABLE IF NOT EXISTS github_app_installations (
    id TEXT PRIMARY KEY NOT NULL,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    installation_id INTEGER NOT NULL UNIQUE,
    account_login TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_github_app_installations_team ON github_app_installations(team_id);
