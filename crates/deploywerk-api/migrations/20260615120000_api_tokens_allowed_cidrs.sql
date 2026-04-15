-- Optional per-token IP allowlist (CIDR strings, JSON array). NULL or [] = no restriction.

ALTER TABLE api_tokens ADD COLUMN IF NOT EXISTS allowed_cidrs JSONB;
