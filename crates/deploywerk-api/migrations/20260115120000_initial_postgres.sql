-- DeployWerk V2 — PostgreSQL baseline (UUID + TIMESTAMPTZ). Replaces former SQLite migrations.

CREATE TABLE users (
    id UUID PRIMARY KEY NOT NULL,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    name TEXT,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE teams (
    id UUID PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE team_memberships (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK(role IN ('owner', 'admin', 'member')),
    PRIMARY KEY (user_id, team_id)
);

CREATE INDEX idx_team_memberships_team ON team_memberships(team_id);
CREATE INDEX idx_team_memberships_user ON team_memberships(user_id);

CREATE TABLE user_preferences (
    user_id UUID PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    current_team_id UUID REFERENCES teams(id) ON DELETE SET NULL
);

CREATE TABLE projects (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    UNIQUE(team_id, slug)
);

CREATE TABLE environments (
    id UUID PRIMARY KEY NOT NULL,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    UNIQUE(project_id, slug)
);

CREATE INDEX idx_projects_team ON projects(team_id);
CREATE INDEX idx_environments_project ON environments(project_id);

CREATE TABLE api_tokens (
    id UUID PRIMARY KEY NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    scopes TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_api_tokens_user ON api_tokens(user_id);

CREATE TABLE invitations (
    id UUID PRIMARY KEY NOT NULL,
    token TEXT NOT NULL UNIQUE,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('owner', 'admin', 'member')),
    expires_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_invitations_team ON invitations(team_id);
