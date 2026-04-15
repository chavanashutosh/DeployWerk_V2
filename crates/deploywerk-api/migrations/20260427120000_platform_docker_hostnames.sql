-- Platform Docker destination (local API host) + optional auto-provisioned hostnames.

ALTER TABLE destinations DROP CONSTRAINT IF EXISTS destinations_kind_check;

ALTER TABLE destinations ALTER COLUMN server_id DROP NOT NULL;

ALTER TABLE destinations
    ADD CONSTRAINT destinations_kind_check
    CHECK (kind IN ('docker_standalone', 'docker_platform'));

ALTER TABLE destinations
    ADD CONSTRAINT destinations_server_kind_check
    CHECK (
        (kind = 'docker_platform' AND server_id IS NULL)
        OR (kind = 'docker_standalone' AND server_id IS NOT NULL)
    );

ALTER TABLE applications
    ADD COLUMN auto_hostname TEXT NULL;

CREATE UNIQUE INDEX idx_applications_auto_hostname_unique
    ON applications (auto_hostname)
    WHERE auto_hostname IS NOT NULL;
