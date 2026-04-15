-- Track image history for one-step rollback; allow rollback job kind.

ALTER TABLE applications
    ADD COLUMN IF NOT EXISTS last_deployed_image TEXT NULL,
    ADD COLUMN IF NOT EXISTS previous_deployed_image TEXT NULL;

ALTER TABLE deploy_jobs DROP CONSTRAINT IF EXISTS deploy_jobs_job_kind_check;
ALTER TABLE deploy_jobs ADD CONSTRAINT deploy_jobs_job_kind_check
    CHECK (job_kind IN ('standard', 'pr_preview', 'pr_preview_destroy', 'rollback'));
