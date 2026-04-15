-- Minimal OTLP trace ingest persistence (Phase 4): store raw batches for later processing / explorer UI.
-- This is intentionally simple: it captures payloads and metadata without attempting to parse OTLP.

CREATE TABLE IF NOT EXISTS otlp_trace_batches (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    content_type TEXT NOT NULL DEFAULT '',
    payload BYTEA NOT NULL,
    size_bytes INT NOT NULL,
    received_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_otlp_trace_batches_team_time ON otlp_trace_batches(team_id, received_at DESC);

