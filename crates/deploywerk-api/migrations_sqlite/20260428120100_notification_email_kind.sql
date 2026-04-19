-- Recreate notification_endpoints to widen kind CHECK (SQLite).

PRAGMA foreign_keys = OFF;

CREATE TABLE notification_endpoints_new (
    id TEXT PRIMARY KEY NOT NULL,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('generic_http', 'discord_webhook', 'telegram', 'email')),
    target_url TEXT NOT NULL,
    events TEXT NOT NULL DEFAULT 'deploy_succeeded,deploy_failed',
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    created_at TEXT NOT NULL
);

INSERT INTO notification_endpoints_new SELECT * FROM notification_endpoints;

DROP TABLE notification_endpoints;
ALTER TABLE notification_endpoints_new RENAME TO notification_endpoints;

CREATE INDEX idx_notification_endpoints_team ON notification_endpoints(team_id);

PRAGMA foreign_keys = ON;
