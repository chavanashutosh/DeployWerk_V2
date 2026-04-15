-- Org-level MFA requirement + SAML IdP storage + user TOTP secrets (Phase 1).

ALTER TABLE organizations
    ADD COLUMN IF NOT EXISTS mfa_required BOOLEAN NOT NULL DEFAULT FALSE;

-- User-scoped TOTP secret for local-password accounts.
CREATE TABLE IF NOT EXISTS user_totp (
    user_id UUID PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    secret_ciphertext BYTEA NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

-- Organization-scoped SAML IdP metadata blobs (insecure parsing MVP; signature verification is future work).
CREATE TABLE IF NOT EXISTS saml_identity_providers (
    id UUID PRIMARY KEY NOT NULL,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    metadata_xml TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_saml_idps_org ON saml_identity_providers(organization_id);

