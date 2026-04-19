CREATE TABLE application_memberships (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('admin', 'viewer')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (user_id, application_id)
);

CREATE INDEX idx_application_memberships_application_id ON application_memberships(application_id);
CREATE INDEX idx_application_memberships_user_id ON application_memberships(user_id);
