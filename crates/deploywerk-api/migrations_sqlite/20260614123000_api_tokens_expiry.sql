ALTER TABLE api_tokens ADD COLUMN expires_at TEXT NULL;

CREATE INDEX IF NOT EXISTS idx_api_tokens_user_expires_at ON api_tokens(user_id, expires_at);
