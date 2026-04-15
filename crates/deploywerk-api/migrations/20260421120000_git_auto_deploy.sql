-- Git push auto-deploy: match repo + branch pattern per application; optional team webhook secret

ALTER TABLE teams ADD COLUMN IF NOT EXISTS github_webhook_secret TEXT;

ALTER TABLE applications
    ADD COLUMN IF NOT EXISTS git_repo_full_name TEXT,
    ADD COLUMN IF NOT EXISTS auto_deploy_on_push BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS git_branch_pattern TEXT NOT NULL DEFAULT 'main';

CREATE INDEX IF NOT EXISTS idx_applications_git_repo ON applications (git_repo_full_name)
    WHERE git_repo_full_name IS NOT NULL AND auto_deploy_on_push = TRUE;

ALTER TABLE deploy_jobs
    ADD COLUMN IF NOT EXISTS git_ref TEXT,
    ADD COLUMN IF NOT EXISTS git_sha TEXT;
