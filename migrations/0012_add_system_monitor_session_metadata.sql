-- Keystone-compatible monitor online-user metadata.
-- Agora uses refresh_tokens as the authoritative session store, so these
-- passive columns let /monitor/onlineUsers expose login context without Redis.

ALTER TABLE refresh_tokens ADD COLUMN IF NOT EXISTS login_ip VARCHAR(128);
ALTER TABLE refresh_tokens ADD COLUMN IF NOT EXISTS login_location VARCHAR(255);
ALTER TABLE refresh_tokens ADD COLUMN IF NOT EXISTS browser VARCHAR(50);
ALTER TABLE refresh_tokens ADD COLUMN IF NOT EXISTS operation_system VARCHAR(50);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_token_prefix
    ON refresh_tokens (left(token, 16));
