-- Optional hash-chain pointer for tamper-evidence (filled by application logic over time).

ALTER TABLE admin_audit_log ADD COLUMN IF NOT EXISTS chain_prev_hash TEXT NULL;
