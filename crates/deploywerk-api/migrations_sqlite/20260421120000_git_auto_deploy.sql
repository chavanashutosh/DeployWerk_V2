ALTER TABLE teams ADD COLUMN github_webhook_secret TEXT;

ALTER TABLE applications ADD COLUMN git_repo_full_name TEXT;
ALTER TABLE applications ADD COLUMN auto_deploy_on_push INTEGER NOT NULL DEFAULT 0 CHECK (auto_deploy_on_push IN (0, 1));
ALTER TABLE applications ADD COLUMN git_branch_pattern TEXT NOT NULL DEFAULT 'main';

CREATE INDEX IF NOT EXISTS idx_applications_git_repo ON applications (git_repo_full_name)
    WHERE git_repo_full_name IS NOT NULL AND auto_deploy_on_push = 1;

ALTER TABLE deploy_jobs ADD COLUMN git_ref TEXT;
ALTER TABLE deploy_jobs ADD COLUMN git_sha TEXT;
