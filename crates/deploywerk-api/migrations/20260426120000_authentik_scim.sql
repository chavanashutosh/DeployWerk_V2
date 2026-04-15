-- SSO (Authentik OIDC) and SCIM group registry for Authentik-driven RBAC

ALTER TABLE users
    ALTER COLUMN password_hash DROP NOT NULL;

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS idp_issuer TEXT,
    ADD COLUMN IF NOT EXISTS idp_subject TEXT;

-- One SSO identity per (issuer, subject). Legacy password users keep idp_* NULL.
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_idp_issuer_subject
    ON users (idp_issuer, idp_subject)
    WHERE idp_issuer IS NOT NULL AND idp_subject IS NOT NULL;

CREATE TABLE IF NOT EXISTS scim_groups (
    id UUID PRIMARY KEY NOT NULL,
    display_name TEXT NOT NULL UNIQUE
);

CREATE INDEX IF NOT EXISTS idx_scim_groups_display_name ON scim_groups (display_name);
