CREATE INDEX IF NOT EXISTS idx_projects_team_name ON projects(team_id, name);

CREATE INDEX IF NOT EXISTS idx_environments_project_name ON environments(project_id, name);

CREATE INDEX IF NOT EXISTS idx_invitations_team_created_at
  ON invitations(team_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_api_tokens_user_created_at
  ON api_tokens(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_deploy_jobs_queued_created_at
  ON deploy_jobs(created_at ASC)
  WHERE status = 'queued';
