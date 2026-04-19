-- SSO: password_hash nullable; idp columns. Recreate users (SQLite has no DROP NOT NULL).

PRAGMA foreign_keys = OFF;

CREATE TABLE users_new (
    id TEXT PRIMARY KEY NOT NULL,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT,
    name TEXT,
    created_at TEXT NOT NULL,
    is_platform_admin INTEGER NOT NULL DEFAULT 0 CHECK (is_platform_admin IN (0, 1)),
    idp_issuer TEXT,
    idp_subject TEXT
);

INSERT INTO users_new (id, email, password_hash, name, created_at, is_platform_admin, idp_issuer, idp_subject)
SELECT id, email, password_hash, name, created_at, is_platform_admin, NULL, NULL FROM users;

DROP TABLE users;
ALTER TABLE users_new RENAME TO users;

CREATE INDEX IF NOT EXISTS idx_users_created_at ON users(created_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_idp_issuer_subject
    ON users (idp_issuer, idp_subject)
    WHERE idp_issuer IS NOT NULL AND idp_subject IS NOT NULL;

CREATE TABLE IF NOT EXISTS scim_groups (
    id TEXT PRIMARY KEY NOT NULL,
    display_name TEXT NOT NULL UNIQUE
);

CREATE INDEX IF NOT EXISTS idx_scim_groups_display_name ON scim_groups (display_name);

PRAGMA foreign_keys = ON;
