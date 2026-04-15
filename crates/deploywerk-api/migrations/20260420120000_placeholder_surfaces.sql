-- Placeholder surfaces: notifications, usage/support, storage, flags, observability, search, edge, sandboxes, agents, RUM, AI routes, billing, user prefs

CREATE TABLE notification_endpoints (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('generic_http', 'discord_webhook')),
    target_url TEXT NOT NULL,
    events TEXT NOT NULL DEFAULT 'deploy_succeeded,deploy_failed',
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_notification_endpoints_team ON notification_endpoints(team_id);

CREATE TABLE team_support_links (
    team_id UUID PRIMARY KEY NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    docs_url TEXT,
    status_url TEXT,
    contact_email TEXT,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE storage_backends (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    endpoint_url TEXT NOT NULL,
    bucket TEXT NOT NULL,
    region TEXT NOT NULL DEFAULT '',
    path_style BOOLEAN NOT NULL DEFAULT TRUE,
    access_key_ciphertext BYTEA NOT NULL,
    secret_key_ciphertext BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_storage_backends_team ON storage_backends(team_id);

CREATE TABLE feature_flags (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    environment_id UUID REFERENCES environments(id) ON DELETE CASCADE,
    flag_key TEXT NOT NULL,
    value_json JSONB NOT NULL DEFAULT '{}',
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE UNIQUE INDEX idx_feature_flags_team_key_global ON feature_flags (team_id, flag_key) WHERE environment_id IS NULL;
CREATE UNIQUE INDEX idx_feature_flags_team_env_key ON feature_flags (team_id, environment_id, flag_key) WHERE environment_id IS NOT NULL;
CREATE INDEX idx_feature_flags_team ON feature_flags(team_id);

CREATE TABLE health_checks (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    target_url TEXT NOT NULL,
    interval_seconds INT NOT NULL DEFAULT 60 CHECK (interval_seconds >= 15 AND interval_seconds <= 86400),
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_health_checks_team ON health_checks(team_id);

CREATE TABLE health_check_results (
    id UUID PRIMARY KEY NOT NULL,
    check_id UUID NOT NULL REFERENCES health_checks(id) ON DELETE CASCADE,
    ok BOOLEAN NOT NULL,
    latency_ms INT,
    error_message TEXT,
    checked_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_health_check_results_check ON health_check_results(check_id);
CREATE INDEX idx_health_check_results_checked ON health_check_results(checked_at DESC);

CREATE TABLE team_firewall_rules (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    label TEXT NOT NULL DEFAULT '',
    cidr TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_team_firewall_rules_team ON team_firewall_rules(team_id);

CREATE TABLE cdn_purge_requests (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    paths TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued', 'done', 'error')),
    detail TEXT,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_cdn_purge_team ON cdn_purge_requests(team_id);

CREATE TABLE preview_deployments (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    branch TEXT NOT NULL DEFAULT '',
    commit_sha TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'torn_down', 'error')),
    meta JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_preview_deployments_team ON preview_deployments(team_id);

CREATE TABLE team_agents (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    version TEXT,
    meta JSONB NOT NULL DEFAULT '{}',
    last_seen_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_team_agents_team ON team_agents(team_id);

CREATE TABLE rum_events (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    page_path TEXT NOT NULL DEFAULT '/',
    metric_name TEXT NOT NULL,
    metric_value DOUBLE PRECISION NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_rum_events_team_time ON rum_events(team_id, recorded_at DESC);

CREATE TABLE ai_gateway_routes (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    path_prefix TEXT NOT NULL,
    upstream_url TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_ai_gateway_routes_team ON ai_gateway_routes(team_id);

CREATE TABLE team_billing (
    team_id UUID PRIMARY KEY NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    stripe_customer_id TEXT,
    plan_name TEXT NOT NULL DEFAULT 'free',
    status TEXT NOT NULL DEFAULT 'inactive',
    updated_at TIMESTAMPTZ NOT NULL
);

ALTER TABLE user_preferences ADD COLUMN IF NOT EXISTS settings_json JSONB NOT NULL DEFAULT '{}';

ALTER TABLE teams ADD COLUMN IF NOT EXISTS rum_ingest_secret TEXT;
