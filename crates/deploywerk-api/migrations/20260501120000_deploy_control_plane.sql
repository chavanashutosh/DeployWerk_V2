-- Deploy control plane: environment freeze + schedule, application strategy & approvals, job lifecycle.

ALTER TABLE environments
    ADD COLUMN IF NOT EXISTS deploy_locked BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS deploy_lock_reason TEXT NULL,
    ADD COLUMN IF NOT EXISTS deploy_schedule_json TEXT NULL;

ALTER TABLE applications
    ADD COLUMN IF NOT EXISTS deploy_strategy TEXT NOT NULL DEFAULT 'standard',
    ADD COLUMN IF NOT EXISTS require_deploy_approval BOOLEAN NOT NULL DEFAULT FALSE;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'applications_deploy_strategy_check'
    ) THEN
        ALTER TABLE applications ADD CONSTRAINT applications_deploy_strategy_check
            CHECK (deploy_strategy IN ('standard', 'blue_green', 'canary', 'rolling'));
    END IF;
END $$;

ALTER TABLE deploy_jobs
    ADD COLUMN IF NOT EXISTS deploy_strategy TEXT NOT NULL DEFAULT 'standard',
    ADD COLUMN IF NOT EXISTS approved_at TIMESTAMPTZ NULL,
    ADD COLUMN IF NOT EXISTS approved_by_user_id UUID NULL REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE deploy_jobs DROP CONSTRAINT IF EXISTS deploy_jobs_status_check;
ALTER TABLE deploy_jobs ADD CONSTRAINT deploy_jobs_status_check
    CHECK (status IN ('pending_approval', 'queued', 'running', 'succeeded', 'failed'));
