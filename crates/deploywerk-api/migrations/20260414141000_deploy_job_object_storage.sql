-- Deploy job object storage keys (MinIO/S3).

ALTER TABLE deploy_jobs
  ADD COLUMN IF NOT EXISTS log_object_key TEXT,
  ADD COLUMN IF NOT EXISTS artifact_manifest_key TEXT;

