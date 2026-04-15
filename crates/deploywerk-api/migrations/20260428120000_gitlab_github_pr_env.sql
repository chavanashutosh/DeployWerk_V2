-- GitLab push webhooks, GitHub App PR previews, optional destroy jobs

ALTER TABLE teams ADD COLUMN IF NOT EXISTS gitlab_webhook_secret TEXT;

ALTER TABLE applications
    ADD COLUMN IF NOT EXISTS pr_preview_enabled BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE deploy_jobs
    ADD COLUMN IF NOT EXISTS job_kind TEXT NOT NULL DEFAULT 'standard',
    ADD COLUMN IF NOT EXISTS pr_number INTEGER NULL;

CREATE TABLE IF NOT EXISTS github_app_installations (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    installation_id BIGINT NOT NULL UNIQUE,
    account_login TEXT,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_github_app_installations_team ON github_app_installations(team_id);
