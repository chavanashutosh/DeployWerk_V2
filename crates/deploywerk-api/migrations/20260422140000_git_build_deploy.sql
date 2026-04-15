-- Optional on-server image build from git_repo_url (clone + docker build + run).

ALTER TABLE applications
    ADD COLUMN IF NOT EXISTS build_image_from_git BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS git_build_ref TEXT NOT NULL DEFAULT 'main',
    ADD COLUMN IF NOT EXISTS dockerfile_path TEXT NOT NULL DEFAULT 'Dockerfile';
