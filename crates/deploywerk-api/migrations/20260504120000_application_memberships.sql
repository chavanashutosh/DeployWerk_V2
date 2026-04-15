-- Per-application RBAC (SCIM: deploywerk-app-{application_uuid}-admin|viewer).

CREATE TABLE application_memberships (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    application_id UUID NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('admin', 'viewer')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, application_id)
);

CREATE INDEX idx_application_memberships_application_id ON application_memberships(application_id);
CREATE INDEX idx_application_memberships_user_id ON application_memberships(user_id);
