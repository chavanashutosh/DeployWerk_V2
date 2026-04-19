-- Organizations parent of teams; per-org unique team slugs (SQLite rebuild).

PRAGMA foreign_keys = OFF;

CREATE TABLE organizations (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL
);

CREATE TABLE organization_memberships (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK(role IN ('owner', 'admin', 'member')),
    PRIMARY KEY (user_id, organization_id)
);

CREATE INDEX idx_organization_memberships_org ON organization_memberships(organization_id);

INSERT INTO organizations (id, name, slug, created_at)
SELECT id, name, slug, created_at FROM teams;

CREATE TABLE teams_new (
    id TEXT PRIMARY KEY NOT NULL,
    organization_id TEXT NOT NULL REFERENCES organizations(id),
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    created_at TEXT NOT NULL,
    rum_ingest_secret TEXT,
    github_webhook_secret TEXT,
    UNIQUE(organization_id, slug)
);

INSERT INTO teams_new (id, organization_id, name, slug, created_at, rum_ingest_secret, github_webhook_secret)
SELECT t.id, o.id, t.name, t.slug, t.created_at, t.rum_ingest_secret, t.github_webhook_secret
FROM teams t
JOIN organizations o ON o.id = t.id;

CREATE TEMP TABLE _team_memberships_backup AS SELECT * FROM team_memberships;
CREATE TEMP TABLE _invitations_backup AS SELECT * FROM invitations;

DROP TABLE team_memberships;
DROP TABLE invitations;
DROP TABLE teams;

ALTER TABLE teams_new RENAME TO teams;

CREATE TABLE team_memberships (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK(role IN ('owner', 'admin', 'member')),
    PRIMARY KEY (user_id, team_id)
);
CREATE INDEX idx_team_memberships_team ON team_memberships(team_id);
CREATE INDEX idx_team_memberships_user ON team_memberships(user_id);

INSERT INTO team_memberships SELECT * FROM _team_memberships_backup;

CREATE TABLE invitations (
    id TEXT PRIMARY KEY NOT NULL,
    token TEXT NOT NULL UNIQUE,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('owner', 'admin', 'member')),
    expires_at TEXT NOT NULL,
    accepted_at TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_invitations_team ON invitations(team_id);

INSERT INTO invitations SELECT * FROM _invitations_backup;

INSERT INTO organization_memberships (user_id, organization_id, role)
SELECT tm.user_id, t.organization_id, tm.role
FROM team_memberships tm
JOIN teams t ON t.id = tm.team_id;

ALTER TABLE user_preferences ADD COLUMN current_organization_id TEXT REFERENCES organizations(id) ON DELETE SET NULL;

UPDATE user_preferences
SET current_organization_id = (
    SELECT t.organization_id FROM teams t WHERE t.id = user_preferences.current_team_id
)
WHERE current_team_id IS NOT NULL
  AND current_organization_id IS NULL;

PRAGMA foreign_keys = ON;
