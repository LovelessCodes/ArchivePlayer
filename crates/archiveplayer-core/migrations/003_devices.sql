-- Browser sessions gain metadata so native clients and browsers can share the
-- same session table. Existing sessions are backfilled as full-access browsers.
ALTER TABLE sessions ADD COLUMN public_id TEXT;
ALTER TABLE sessions ADD COLUMN kind TEXT NOT NULL DEFAULT 'browser';
ALTER TABLE sessions ADD COLUMN label TEXT;
ALTER TABLE sessions ADD COLUMN platform TEXT;
ALTER TABLE sessions ADD COLUMN scopes TEXT NOT NULL DEFAULT '["*"]';
ALTER TABLE sessions ADD COLUMN last_seen_at INTEGER;

UPDATE sessions
SET public_id = lower(hex(randomblob(8)))
WHERE public_id IS NULL;

UPDATE sessions
SET last_seen_at = CAST(strftime('%s', 'now') AS INTEGER)
WHERE last_seen_at IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_sessions_public_id ON sessions(public_id);

-- A pending pairing request. Both codes are stored as SHA-256 hashes; the raw
-- device code is only known to the client and the user code only to the person
-- approving the device.
CREATE TABLE IF NOT EXISTS device_authorizations (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    device_code_hash TEXT NOT NULL UNIQUE,
    user_code_hash   TEXT NOT NULL UNIQUE,
    client_name      TEXT NOT NULL,
    platform         TEXT NOT NULL,
    scopes           TEXT NOT NULL,
    status           TEXT NOT NULL DEFAULT 'pending',
    user_id          INTEGER REFERENCES users(id) ON DELETE CASCADE,
    interval_seconds INTEGER NOT NULL DEFAULT 5,
    last_polled_at   INTEGER,
    expires_at       INTEGER NOT NULL,
    created_at       TEXT NOT NULL,
    approved_at      TEXT,
    consumed_at      TEXT
);

CREATE INDEX IF NOT EXISTS idx_device_auth_expires ON device_authorizations(expires_at);
CREATE INDEX IF NOT EXISTS idx_device_auth_user_code ON device_authorizations(user_code_hash);

PRAGMA user_version = 3;
