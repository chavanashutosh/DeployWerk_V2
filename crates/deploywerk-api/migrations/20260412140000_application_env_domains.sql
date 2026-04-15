-- Env vars, public domains metadata, optional Git URL (P3 breadth).

ALTER TABLE applications
    ADD COLUMN domains JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN git_repo_url TEXT;

CREATE TABLE application_env_vars (
    id UUID PRIMARY KEY NOT NULL,
    application_id UUID NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    is_secret BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL,
    UNIQUE(application_id, key)
);

CREATE INDEX idx_application_env_vars_app ON application_env_vars(application_id);
