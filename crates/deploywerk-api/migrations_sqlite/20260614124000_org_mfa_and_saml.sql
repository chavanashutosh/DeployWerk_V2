ALTER TABLE organizations ADD COLUMN mfa_required INTEGER NOT NULL DEFAULT 0 CHECK (mfa_required IN (0, 1));

CREATE TABLE IF NOT EXISTS user_totp (
    user_id TEXT PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    secret_ciphertext BLOB NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS saml_identity_providers (
    id TEXT PRIMARY KEY NOT NULL,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    metadata_xml TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_saml_idps_org ON saml_identity_providers(organization_id);
