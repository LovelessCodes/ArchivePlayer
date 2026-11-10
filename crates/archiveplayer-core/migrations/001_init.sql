PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS users (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    email         TEXT NOT NULL UNIQUE COLLATE NOCASE,
    display_name  TEXT,
    password_hash  TEXT NOT NULL,
    created_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS media_items (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    identifier      TEXT NOT NULL UNIQUE,
    direct_url      TEXT,
    title           TEXT NOT NULL,
    description     TEXT,
    thumbnail_url   TEXT,
    media_type      TEXT NOT NULL DEFAULT 'Unknown',
    tmdb_poster_url TEXT,
    tmdb_id         INTEGER,
    tmdb_media_type TEXT,
    created_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS user_media_items (
    id                        INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id                   INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    media_item_id             INTEGER NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    category_name             TEXT NOT NULL DEFAULT 'Uncategorized',
    watch_progress_seconds    INTEGER NOT NULL DEFAULT 0,
    duration_seconds          INTEGER NOT NULL DEFAULT 0,
    is_completed              INTEGER NOT NULL DEFAULT 0,
    autoplay_enabled          INTEGER NOT NULL DEFAULT 1,
    last_watched              TEXT,
    current_episode_file_name TEXT,
    added_at                  TEXT NOT NULL,
    UNIQUE (user_id, media_item_id)
);

CREATE TABLE IF NOT EXISTS episode_progress (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    user_media_item_id  INTEGER NOT NULL REFERENCES user_media_items(id) ON DELETE CASCADE,
    file_name           TEXT NOT NULL,
    position_seconds    INTEGER NOT NULL DEFAULT 0,
    duration_seconds    INTEGER NOT NULL DEFAULT 0,
    completed           INTEGER NOT NULL DEFAULT 0,
    updated_at          TEXT NOT NULL,
    UNIQUE (user_media_item_id, file_name)
);

CREATE INDEX IF NOT EXISTS idx_user_media_user ON user_media_items(user_id);
CREATE INDEX IF NOT EXISTS idx_user_media_recent ON user_media_items(user_id, last_watched DESC);
CREATE INDEX IF NOT EXISTS idx_episode_progress_item ON episode_progress(user_media_item_id);

PRAGMA user_version = 1;
