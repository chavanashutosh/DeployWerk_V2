-- Runtime persistent volume mounts for applications.

ALTER TABLE applications
  ADD COLUMN IF NOT EXISTS runtime_volumes_json JSONB NOT NULL DEFAULT '[]'::jsonb;

