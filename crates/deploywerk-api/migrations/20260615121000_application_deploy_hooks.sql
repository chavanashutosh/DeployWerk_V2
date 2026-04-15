-- Pre/post deploy HTTP hooks (POST JSON); invoked by deploy worker around container replace.

ALTER TABLE applications ADD COLUMN IF NOT EXISTS pre_deploy_hook_url TEXT,
    ADD COLUMN IF NOT EXISTS post_deploy_hook_url TEXT;
