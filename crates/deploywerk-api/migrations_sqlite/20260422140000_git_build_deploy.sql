ALTER TABLE applications ADD COLUMN build_image_from_git INTEGER NOT NULL DEFAULT 0 CHECK (build_image_from_git IN (0, 1));
ALTER TABLE applications ADD COLUMN git_build_ref TEXT NOT NULL DEFAULT 'main';
ALTER TABLE applications ADD COLUMN dockerfile_path TEXT NOT NULL DEFAULT 'Dockerfile';
