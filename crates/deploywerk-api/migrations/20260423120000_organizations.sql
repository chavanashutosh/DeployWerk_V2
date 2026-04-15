-- Organizations parent of teams; per-org unique team slugs.

CREATE TABLE organizations (
    id UUID PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE organization_memberships (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK(role IN ('owner', 'admin', 'member')),
    PRIMARY KEY (user_id, organization_id)
);

CREATE INDEX idx_organization_memberships_org ON organization_memberships(organization_id);

ALTER TABLE teams ADD COLUMN organization_id UUID REFERENCES organizations(id);

-- One org per existing team; copy memberships from team_memberships.
CREATE TEMP TABLE _team_org_map (team_id UUID PRIMARY KEY, org_id UUID NOT NULL);
INSERT INTO _team_org_map (team_id, org_id) SELECT id, gen_random_uuid() FROM teams;

INSERT INTO organizations (id, name, slug, created_at)
SELECT m.org_id, t.name, t.slug, t.created_at
FROM _team_org_map m
JOIN teams t ON t.id = m.team_id;

UPDATE teams t
SET organization_id = m.org_id
FROM _team_org_map m
WHERE t.id = m.team_id;

INSERT INTO organization_memberships (user_id, organization_id, role)
SELECT tm.user_id, m.org_id, tm.role
FROM team_memberships tm
JOIN _team_org_map m ON m.team_id = tm.team_id;

ALTER TABLE teams ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE teams DROP CONSTRAINT IF EXISTS teams_slug_key;

ALTER TABLE teams ADD CONSTRAINT teams_organization_id_slug_key UNIQUE (organization_id, slug);

ALTER TABLE user_preferences
    ADD COLUMN IF NOT EXISTS current_organization_id UUID REFERENCES organizations(id) ON DELETE SET NULL;

UPDATE user_preferences up
SET current_organization_id = t.organization_id
FROM teams t
WHERE up.current_team_id IS NOT NULL
  AND up.current_team_id = t.id
  AND up.current_organization_id IS NULL;
