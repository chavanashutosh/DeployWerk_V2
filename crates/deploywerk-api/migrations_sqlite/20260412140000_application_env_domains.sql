ALTER TABLE applications ADD COLUMN domains TEXT NOT NULL DEFAULT '[]';
ALTER TABLE applications ADD COLUMN git_repo_url TEXT;

CREATE TABLE application_env_vars (
    id TEXT PRIMARY KEY NOT NULL,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    is_secret INTEGER NOT NULL DEFAULT 0 CHECK (is_secret IN (0, 1)),
    created_at TEXT NOT NULL,
    UNIQUE(application_id, key)
);

CREATE INDEX idx_application_env_vars_app ON application_env_vars(application_id);
