-- Mail platform Phase 1 (schema scaffold): domains + transactional messages.

CREATE TABLE IF NOT EXISTS mail_domains (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    domain TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','active','disabled')),
    created_at TIMESTAMPTZ NOT NULL,
    UNIQUE(team_id, domain)
);

CREATE INDEX IF NOT EXISTS idx_mail_domains_team ON mail_domains(team_id);

CREATE TABLE IF NOT EXISTS mail_messages (
    id UUID PRIMARY KEY NOT NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    from_addr TEXT NOT NULL,
    to_addrs JSONB NOT NULL DEFAULT '[]'::jsonb,
    subject TEXT NOT NULL DEFAULT '',
    text_body TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued','sent','error')),
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    sent_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_mail_messages_team_time ON mail_messages(team_id, created_at DESC);

