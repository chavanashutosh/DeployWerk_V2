CREATE TABLE IF NOT EXISTS otlp_trace_batches (
    id TEXT PRIMARY KEY NOT NULL,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    content_type TEXT NOT NULL DEFAULT '',
    payload BLOB NOT NULL,
    size_bytes INTEGER NOT NULL,
    received_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_otlp_trace_batches_team_time ON otlp_trace_batches(team_id, received_at DESC);
