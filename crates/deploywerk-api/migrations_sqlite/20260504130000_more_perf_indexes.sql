CREATE INDEX IF NOT EXISTS idx_deploy_jobs_application_created_at
  ON deploy_jobs(application_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_applications_environment_created ON applications(environment_id, created_at DESC);
