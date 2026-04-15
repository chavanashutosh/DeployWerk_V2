-- Allow telegram (already used in API) and email notification endpoints.
ALTER TABLE notification_endpoints DROP CONSTRAINT IF EXISTS notification_endpoints_kind_check;
ALTER TABLE notification_endpoints ADD CONSTRAINT notification_endpoints_kind_check
  CHECK (kind IN ('generic_http', 'discord_webhook', 'telegram', 'email'));
