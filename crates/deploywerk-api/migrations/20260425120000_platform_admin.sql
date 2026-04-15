-- Platform operator admin: roles, audit, billing provider fields, entitlements, billing event log

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS is_platform_admin BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE IF NOT EXISTS admin_audit_log (
    id UUID PRIMARY KEY NOT NULL,
    actor_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id UUID,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_admin_audit_log_created ON admin_audit_log(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_admin_audit_log_actor ON admin_audit_log(actor_user_id);

ALTER TABLE team_billing
    ADD COLUMN IF NOT EXISTS payment_provider TEXT NOT NULL DEFAULT 'none',
    ADD COLUMN IF NOT EXISTS provider_customer_id TEXT,
    ADD COLUMN IF NOT EXISTS billing_sync_json JSONB NOT NULL DEFAULT '{}';

UPDATE team_billing
SET provider_customer_id = stripe_customer_id
WHERE provider_customer_id IS NULL AND stripe_customer_id IS NOT NULL;

UPDATE team_billing
SET payment_provider = 'stripe'
WHERE payment_provider = 'none' AND stripe_customer_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS billing_events (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID REFERENCES teams(id) ON DELETE SET NULL,
    provider TEXT NOT NULL,
    event_code TEXT NOT NULL DEFAULT '',
    psp_reference TEXT,
    merchant_reference TEXT,
    payload JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_billing_events_team_time ON billing_events(team_id, created_at DESC);

CREATE TABLE IF NOT EXISTS platform_feature_definitions (
    feature_key TEXT PRIMARY KEY NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    default_on BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS team_entitlements (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    feature_key TEXT NOT NULL REFERENCES platform_feature_definitions(feature_key) ON DELETE CASCADE,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    source TEXT NOT NULL CHECK (source IN ('plan', 'manual', 'trial')),
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    UNIQUE(team_id, feature_key)
);

CREATE INDEX IF NOT EXISTS idx_team_entitlements_team ON team_entitlements(team_id);

INSERT INTO platform_feature_definitions (feature_key, description, default_on, created_at)
VALUES
    ('ai_gateway', 'AI Gateway routes and proxy invoke', TRUE, NOW()),
    ('rum', 'RUM ingest and summary APIs', TRUE, NOW())
ON CONFLICT (feature_key) DO NOTHING;

CREATE INDEX IF NOT EXISTS idx_deploy_jobs_created_at ON deploy_jobs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_users_created_at ON users(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_teams_created_at ON teams(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_rum_events_recorded_at ON rum_events(recorded_at DESC);
