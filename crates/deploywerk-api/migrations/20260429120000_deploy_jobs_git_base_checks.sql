-- Base SHA for PR compare links; constrain job_kind; list index for application timelines.

ALTER TABLE deploy_jobs ADD COLUMN IF NOT EXISTS git_base_sha TEXT NULL;

ALTER TABLE deploy_jobs DROP CONSTRAINT IF EXISTS deploy_jobs_job_kind_check;
ALTER TABLE deploy_jobs ADD CONSTRAINT deploy_jobs_job_kind_check
  CHECK (job_kind IN ('standard', 'pr_preview', 'pr_preview_destroy'));

CREATE INDEX IF NOT EXISTS idx_deploy_jobs_application_created_at
  ON deploy_jobs (application_id, created_at DESC);
