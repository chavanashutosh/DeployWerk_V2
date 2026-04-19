CREATE TABLE IF NOT EXISTS mail_domains (
    id TEXT PRIMARY KEY NOT NULL,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    domain TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','active','disabled')),
    created_at TEXT NOT NULL,
    UNIQUE(team_id, domain)
);

CREATE INDEX IF NOT EXISTS idx_mail_domains_team ON mail_domains(team_id);

CREATE TABLE IF NOT EXISTS mail_messages (
    id TEXT PRIMARY KEY NOT NULL,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    from_addr TEXT NOT NULL,
    to_addrs TEXT NOT NULL DEFAULT '[]',
    subject TEXT NOT NULL DEFAULT '',
    text_body TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued','sent','error')),
    error_message TEXT,
    created_at TEXT NOT NULL,
    sent_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_mail_messages_team_time ON mail_messages(team_id, created_at DESC);
