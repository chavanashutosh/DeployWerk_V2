-- API token expiry (P1): optional expires_at timestamp.

ALTER TABLE api_tokens
    ADD COLUMN IF NOT EXISTS expires_at TIMESTAMPTZ NULL;

CREATE INDEX IF NOT EXISTS idx_api_tokens_user_expires_at ON api_tokens(user_id, expires_at);

