use crate::error::{CoreError, CoreResult};
use crate::models::{CategoryGroup, EpisodeProgress, LibraryItem, LibraryResponse, User};
use crate::util::now_rfc3339;
use rusqlite::{Connection, OptionalExtension, Row, params};
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

const SCHEMA: &str = include_str!("../migrations/001_init.sql");
const SESSION_SCHEMA: &str = include_str!("../migrations/002_sessions.sql");
const DEVICE_SCHEMA: &str = include_str!("../migrations/003_devices.sql");

const LIBRARY_SELECT: &str = r#"
    SELECT
        umi.id,
        mi.identifier,
        mi.title,
        mi.media_type,
        mi.thumbnail_url,
        mi.tmdb_poster_url,
        umi.category_name,
        umi.watch_progress_seconds,
        umi.duration_seconds,
        umi.current_episode_file_name,
        umi.last_watched,
        umi.is_completed,
        umi.added_at
    FROM user_media_items umi
    JOIN media_items mi ON mi.id = umi.media_item_id
"#;

/// SQLite-backed application state. A single connection behind a mutex is
/// plenty for a local desktop app and keeps writes serialized.
pub struct Database {
    connection: Mutex<Connection>,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user: User,
    pub password_hash: String,
}

/// Metadata for one signed-in browser or paired device.
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub public_id: String,
    pub user_id: i64,
    pub kind: String,
    pub label: Option<String>,
    pub platform: Option<String>,
    pub scopes: Vec<String>,
    pub created_at: String,
    pub last_seen_at: Option<i64>,
    pub expires_at: i64,
}

#[derive(Debug, Clone)]
pub struct AuthSession {
    pub user: User,
    pub session: SessionRecord,
}

#[derive(Debug, Clone)]
pub struct NewSession {
    pub token_hash: String,
    pub user_id: i64,
    pub expires_at: i64,
    pub kind: String,
    pub label: Option<String>,
    pub platform: Option<String>,
    pub scopes: Vec<String>,
    pub last_seen_at: i64,
}

#[derive(Debug, Clone)]
pub struct NewDeviceAuthorization {
    pub device_code_hash: String,
    pub user_code_hash: String,
    pub client_name: String,
    pub platform: String,
    pub scopes: Vec<String>,
    pub expires_at: i64,
    pub interval_seconds: i64,
}

#[derive(Debug, Clone)]
pub struct DeviceAuthorization {
    pub id: i64,
    pub client_name: String,
    pub platform: String,
    pub scopes: Vec<String>,
    pub status: String,
    pub user_id: Option<i64>,
    pub expires_at: i64,
    pub interval_seconds: i64,
    pub last_polled_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct MediaItemRow {
    pub id: i64,
    pub identifier: String,
    pub title: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub media_type: String,
    pub tmdb_poster_url: Option<String>,
    pub tmdb_id: Option<i64>,
    pub tmdb_media_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewMediaItem {
    pub identifier: String,
    pub title: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub media_type: String,
    pub tmdb_poster_url: Option<String>,
    pub tmdb_id: Option<i64>,
    pub tmdb_media_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserMediaRow {
    pub id: i64,
    pub media_item_id: i64,
    pub category_name: String,
    pub watch_progress_seconds: i64,
    pub duration_seconds: i64,
    pub current_episode_file_name: Option<String>,
    pub last_watched: Option<String>,
    pub is_completed: bool,
    pub autoplay_enabled: bool,
}

fn parse_scopes(raw: &str) -> Vec<String> {
    // A malformed scope list must grant nothing, never full access.
    serde_json::from_str(raw).unwrap_or_default()
}

fn map_session(row: &Row<'_>) -> rusqlite::Result<SessionRecord> {
    Ok(SessionRecord {
        public_id: row.get(0)?,
        user_id: row.get(1)?,
        kind: row.get(2)?,
        label: row.get(3)?,
        platform: row.get(4)?,
        scopes: parse_scopes(&row.get::<_, String>(5)?),
        created_at: row.get(6)?,
        last_seen_at: row.get(7)?,
        expires_at: row.get(8)?,
    })
}

fn map_device_authorization(row: &Row<'_>) -> rusqlite::Result<DeviceAuthorization> {
    Ok(DeviceAuthorization {
        id: row.get(0)?,
        client_name: row.get(1)?,
        platform: row.get(2)?,
        scopes: parse_scopes(&row.get::<_, String>(3)?),
        status: row.get(4)?,
        user_id: row.get(5)?,
        expires_at: row.get(6)?,
        interval_seconds: row.get(7)?,
        last_polled_at: row.get(8)?,
    })
}

impl Database {
    pub fn open(path: &Path) -> CoreResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        }
        Self::initialize(connection)
    }

    pub fn in_memory() -> CoreResult<Self> {
        Self::initialize(Connection::open_in_memory()?)
    }

    fn initialize(connection: Connection) -> CoreResult<Self> {
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version < 1 {
            connection.execute_batch(SCHEMA)?;
        }
        if version < 2 {
            connection.execute_batch(SESSION_SCHEMA)?;
        }
        if version < 3 {
            connection.execute_batch(DEVICE_SCHEMA)?;
        }
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn with_connection<T>(
        &self,
        operation: impl FnOnce(&mut Connection) -> CoreResult<T>,
    ) -> CoreResult<T> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::Message("The local database lock was poisoned.".into()))?;
        operation(&mut connection)
    }

    // ---------------------------------------------------------------- users

    pub fn user_count(&self) -> CoreResult<i64> {
        self.with_connection(|conn| {
            Ok(conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?)
        })
    }

    pub fn create_user(
        &self,
        email: &str,
        display_name: Option<&str>,
        password_hash: &str,
    ) -> CoreResult<User> {
        let email = email.trim().to_lowercase();
        self.with_connection(|conn| {
            let created_at = now_rfc3339();
            conn.execute(
                "INSERT INTO users (email, display_name, password_hash, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![email, display_name, password_hash, created_at],
            )
            .map_err(|error| match error {
                rusqlite::Error::SqliteFailure(code, _)
                    if code.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    CoreError::Conflict("An account with that email already exists.".into())
                }
                other => CoreError::Database(other),
            })?;

            let id = conn.last_insert_rowid();
            Ok(User {
                id,
                email,
                display_name: display_name.map(str::to_string),
                created_at,
            })
        })
    }

    pub fn find_user_by_email(&self, email: &str) -> CoreResult<Option<AuthUser>> {
        let email = email.trim().to_lowercase();
        self.with_connection(|conn| {
            Ok(conn
                .query_row(
                    "SELECT id, email, display_name, password_hash, created_at
                     FROM users WHERE email = ?1",
                    params![email],
                    |row| {
                        Ok(AuthUser {
                            user: User {
                                id: row.get(0)?,
                                email: row.get(1)?,
                                display_name: row.get(2)?,
                                created_at: row.get(4)?,
                            },
                            password_hash: row.get(3)?,
                        })
                    },
                )
                .optional()?)
        })
    }

    pub fn user_by_id(&self, id: i64) -> CoreResult<Option<User>> {
        self.with_connection(|conn| {
            Ok(conn
                .query_row(
                    "SELECT id, email, display_name, created_at FROM users WHERE id = ?1",
                    params![id],
                    |row| {
                        Ok(User {
                            id: row.get(0)?,
                            email: row.get(1)?,
                            display_name: row.get(2)?,
                            created_at: row.get(3)?,
                        })
                    },
                )
                .optional()?)
        })
    }

    /// The first account created owns server-wide settings such as the TMDB key.
    pub fn first_user_id(&self) -> CoreResult<Option<i64>> {
        self.with_connection(|conn| {
            Ok(conn.query_row("SELECT MIN(id) FROM users", [], |row| row.get(0))?)
        })
    }

    // ------------------------------------------------------------- sessions

    pub fn create_session(&self, session: NewSession) -> CoreResult<()> {
        let scopes = serde_json::to_string(&session.scopes)?;
        self.with_connection(|conn| {
            conn.execute(
                "INSERT INTO sessions
                    (public_id, token_hash, user_id, expires_at, created_at, kind, label,
                     platform, scopes, last_seen_at)
                 VALUES (lower(hex(randomblob(8))), ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    session.token_hash,
                    session.user_id,
                    session.expires_at,
                    now_rfc3339(),
                    session.kind,
                    session.label,
                    session.platform,
                    scopes,
                    session.last_seen_at,
                ],
            )?;
            Ok(())
        })
    }

    pub fn session_auth(&self, token_hash: &str, now: i64) -> CoreResult<Option<AuthSession>> {
        self.with_connection(|conn| {
            Ok(conn
                .query_row(
                    "SELECT s.public_id, s.user_id, s.kind, s.label, s.platform, s.scopes,
                            s.created_at, s.last_seen_at, s.expires_at,
                            u.id, u.email, u.display_name, u.created_at
                     FROM sessions s
                     JOIN users u ON u.id = s.user_id
                     WHERE s.token_hash = ?1 AND s.expires_at > ?2",
                    params![token_hash, now],
                    |row| {
                        Ok(AuthSession {
                            session: map_session(row)?,
                            user: User {
                                id: row.get(9)?,
                                email: row.get(10)?,
                                display_name: row.get(11)?,
                                created_at: row.get(12)?,
                            },
                        })
                    },
                )
                .optional()?)
        })
    }

    pub fn session_user(&self, token_hash: &str, now: i64) -> CoreResult<Option<User>> {
        Ok(self
            .session_auth(token_hash, now)?
            .map(|session| session.user))
    }

    /// Avoid a write on every request while still keeping device lists fresh.
    pub fn touch_session(&self, token_hash: &str, now: i64) -> CoreResult<()> {
        self.with_connection(|conn| {
            conn.execute(
                "UPDATE sessions SET last_seen_at = ?2
                 WHERE token_hash = ?1 AND (last_seen_at IS NULL OR last_seen_at < ?2 - 60)",
                params![token_hash, now],
            )?;
            Ok(())
        })
    }

    pub fn list_sessions(&self, user_id: i64, now: i64) -> CoreResult<Vec<SessionRecord>> {
        self.with_connection(|conn| {
            let mut statement = conn.prepare(
                "SELECT public_id, user_id, kind, label, platform, scopes, created_at,
                        last_seen_at, expires_at
                 FROM sessions
                 WHERE user_id = ?1 AND expires_at > ?2
                 ORDER BY COALESCE(last_seen_at, 0) DESC, created_at DESC",
            )?;
            let rows = statement.query_map(params![user_id, now], map_session)?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        })
    }

    pub fn delete_session(&self, token_hash: &str) -> CoreResult<()> {
        self.with_connection(|conn| {
            conn.execute(
                "DELETE FROM sessions WHERE token_hash = ?1",
                params![token_hash],
            )?;
            Ok(())
        })
    }

    pub fn delete_session_by_public_id(&self, user_id: i64, public_id: &str) -> CoreResult<bool> {
        self.with_connection(|conn| {
            let changed = conn.execute(
                "DELETE FROM sessions WHERE user_id = ?1 AND public_id = ?2",
                params![user_id, public_id],
            )?;
            Ok(changed > 0)
        })
    }

    pub fn delete_expired_sessions(&self, now: i64) -> CoreResult<()> {
        self.with_connection(|conn| {
            conn.execute("DELETE FROM sessions WHERE expires_at <= ?1", params![now])?;
            Ok(())
        })
    }

    // ------------------------------------------------------------ device pairing

    pub fn create_device_authorization(
        &self,
        authorization: NewDeviceAuthorization,
    ) -> CoreResult<()> {
        let scopes = serde_json::to_string(&authorization.scopes)?;
        self.with_connection(|conn| {
            conn.execute(
                "INSERT INTO device_authorizations
                    (device_code_hash, user_code_hash, client_name, platform, scopes,
                     interval_seconds, expires_at, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    authorization.device_code_hash,
                    authorization.user_code_hash,
                    authorization.client_name,
                    authorization.platform,
                    scopes,
                    authorization.interval_seconds,
                    authorization.expires_at,
                    now_rfc3339(),
                ],
            )
            .map_err(|error| match error {
                rusqlite::Error::SqliteFailure(code, _)
                    if code.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    CoreError::Conflict("A pairing code collision occurred.".into())
                }
                other => CoreError::Database(other),
            })?;
            Ok(())
        })
    }

    pub fn device_authorization_by_device_code(
        &self,
        device_code_hash: &str,
    ) -> CoreResult<Option<DeviceAuthorization>> {
        self.with_connection(|conn| {
            Ok(conn
                .query_row(
                    "SELECT id, client_name, platform, scopes, status, user_id, expires_at,
                            interval_seconds, last_polled_at
                     FROM device_authorizations WHERE device_code_hash = ?1",
                    params![device_code_hash],
                    map_device_authorization,
                )
                .optional()?)
        })
    }

    pub fn device_authorization_by_user_code(
        &self,
        user_code_hash: &str,
    ) -> CoreResult<Option<DeviceAuthorization>> {
        self.with_connection(|conn| {
            Ok(conn
                .query_row(
                    "SELECT id, client_name, platform, scopes, status, user_id, expires_at,
                            interval_seconds, last_polled_at
                     FROM device_authorizations WHERE user_code_hash = ?1",
                    params![user_code_hash],
                    map_device_authorization,
                )
                .optional()?)
        })
    }

    pub fn set_device_authorization_status(
        &self,
        user_code_hash: &str,
        user_id: i64,
        status: &str,
        now: i64,
    ) -> CoreResult<Option<DeviceAuthorization>> {
        self.with_connection(|conn| {
            let changed = conn.execute(
                "UPDATE device_authorizations
                 SET status = ?3, user_id = ?2, approved_at = ?4
                 WHERE user_code_hash = ?1 AND status = 'pending' AND expires_at > ?5",
                params![user_code_hash, user_id, status, now_rfc3339(), now],
            )?;
            if changed == 0 {
                return Ok(None);
            }
            Ok(conn
                .query_row(
                    "SELECT id, client_name, platform, scopes, status, user_id, expires_at,
                            interval_seconds, last_polled_at
                     FROM device_authorizations WHERE user_code_hash = ?1",
                    params![user_code_hash],
                    map_device_authorization,
                )
                .optional()?)
        })
    }

    pub fn record_device_poll(&self, id: i64, now: i64) -> CoreResult<()> {
        self.with_connection(|conn| {
            conn.execute(
                "UPDATE device_authorizations SET last_polled_at = ?2 WHERE id = ?1",
                params![id, now],
            )?;
            Ok(())
        })
    }

    /// Atomically marks an approved pairing as consumed and creates the device
    /// session. Returning `None` means another poll already redeemed it.
    pub fn redeem_device_authorization(
        &self,
        device_code_hash: &str,
        token_hash: &str,
        expires_at: i64,
        now: i64,
    ) -> CoreResult<Option<User>> {
        self.with_connection(|conn| {
            let transaction = conn.transaction()?;
            let record = transaction
                .query_row(
                    "SELECT id, client_name, platform, scopes, user_id
                     FROM device_authorizations
                     WHERE device_code_hash = ?1 AND status = 'approved' AND expires_at > ?2",
                    params![device_code_hash, now],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, Option<i64>>(4)?,
                        ))
                    },
                )
                .optional()?;
            let Some((id, label, platform, scopes, Some(user_id))) = record else {
                return Ok(None);
            };

            let changed = transaction.execute(
                "UPDATE device_authorizations
                 SET status = 'consumed', consumed_at = ?2
                 WHERE id = ?1 AND status = 'approved'",
                params![id, now_rfc3339()],
            )?;
            if changed == 0 {
                return Ok(None);
            }

            transaction.execute(
                "INSERT INTO sessions
                    (public_id, token_hash, user_id, expires_at, created_at, kind, label,
                     platform, scopes, last_seen_at)
                 VALUES (lower(hex(randomblob(8))), ?1, ?2, ?3, ?4, 'device', ?5, ?6, ?7, ?8)",
                params![
                    token_hash,
                    user_id,
                    expires_at,
                    now_rfc3339(),
                    label,
                    platform,
                    scopes,
                    now,
                ],
            )?;

            let user = transaction.query_row(
                "SELECT id, email, display_name, created_at FROM users WHERE id = ?1",
                params![user_id],
                |row| {
                    Ok(User {
                        id: row.get(0)?,
                        email: row.get(1)?,
                        display_name: row.get(2)?,
                        created_at: row.get(3)?,
                    })
                },
            )?;
            transaction.commit()?;
            Ok(Some(user))
        })
    }

    pub fn delete_expired_device_authorizations(&self, now: i64) -> CoreResult<()> {
        self.with_connection(|conn| {
            conn.execute(
                "DELETE FROM device_authorizations WHERE expires_at <= ?1",
                params![now],
            )?;
            Ok(())
        })
    }

    // --------------------------------------------------------------- media

    pub fn media_by_identifier(&self, identifier: &str) -> CoreResult<Option<MediaItemRow>> {
        self.with_connection(|conn| {
            Ok(conn
                .query_row(
                    "SELECT id, identifier, title, description, thumbnail_url, media_type,
                            tmdb_poster_url, tmdb_id, tmdb_media_type
                     FROM media_items WHERE identifier = ?1",
                    params![identifier],
                    map_media,
                )
                .optional()?)
        })
    }

    pub fn insert_media(&self, media: &NewMediaItem) -> CoreResult<MediaItemRow> {
        self.with_connection(|conn| {
            conn.execute(
                "INSERT INTO media_items (
                    identifier, title, description, thumbnail_url, media_type,
                    tmdb_poster_url, tmdb_id, tmdb_media_type, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(identifier) DO NOTHING",
                params![
                    media.identifier,
                    media.title,
                    media.description,
                    media.thumbnail_url,
                    media.media_type,
                    media.tmdb_poster_url,
                    media.tmdb_id,
                    media.tmdb_media_type,
                    now_rfc3339(),
                ],
            )?;

            conn.query_row(
                "SELECT id, identifier, title, description, thumbnail_url, media_type,
                        tmdb_poster_url, tmdb_id, tmdb_media_type
                 FROM media_items WHERE identifier = ?1",
                params![media.identifier],
                map_media,
            )
            .map_err(CoreError::from)
        })
    }

    pub fn update_media(&self, media: &MediaItemRow) -> CoreResult<()> {
        self.with_connection(|conn| {
            conn.execute(
                "UPDATE media_items SET
                    title = ?2,
                    description = COALESCE(?3, description),
                    thumbnail_url = COALESCE(?4, thumbnail_url),
                    media_type = ?5,
                    tmdb_poster_url = COALESCE(?6, tmdb_poster_url),
                    tmdb_id = COALESCE(?7, tmdb_id),
                    tmdb_media_type = COALESCE(?8, tmdb_media_type)
                 WHERE id = ?1",
                params![
                    media.id,
                    media.title,
                    media.description,
                    media.thumbnail_url,
                    media.media_type,
                    media.tmdb_poster_url,
                    media.tmdb_id,
                    media.tmdb_media_type,
                ],
            )?;
            Ok(())
        })
    }

    // ------------------------------------------------------------- library

    pub fn library_for_user(&self, user_id: i64) -> CoreResult<LibraryResponse> {
        self.with_connection(|conn| {
            let mut statement = conn.prepare(&format!(
                "{LIBRARY_SELECT} WHERE umi.user_id = ?1 ORDER BY umi.added_at DESC, mi.title COLLATE NOCASE"
            ))?;
            let items = statement
                .query_map(params![user_id], map_library_item)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(group_library(items))
        })
    }

    pub fn continue_watching(&self, user_id: i64, limit: i64) -> CoreResult<Vec<LibraryItem>> {
        self.with_connection(|conn| {
            let mut statement = conn.prepare(&format!(
                "{LIBRARY_SELECT}
                 WHERE umi.user_id = ?1
                   AND umi.watch_progress_seconds > 0
                   AND umi.is_completed = 0
                   AND umi.last_watched IS NOT NULL
                 ORDER BY umi.last_watched DESC
                 LIMIT ?2"
            ))?;
            let items = statement
                .query_map(params![user_id, limit], map_library_item)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(items)
        })
    }

    pub fn library_item_by_id(&self, user_id: i64, id: i64) -> CoreResult<Option<LibraryItem>> {
        self.with_connection(|conn| {
            Ok(conn
                .query_row(
                    &format!("{LIBRARY_SELECT} WHERE umi.user_id = ?1 AND umi.id = ?2"),
                    params![user_id, id],
                    map_library_item,
                )
                .optional()?)
        })
    }

    pub fn add_user_media(
        &self,
        user_id: i64,
        media_item_id: i64,
        category_name: &str,
    ) -> CoreResult<LibraryItem> {
        let category = if category_name.trim().is_empty() {
            "Uncategorized"
        } else {
            category_name.trim()
        };

        let id = self.with_connection(|conn| {
            conn.execute(
                "INSERT INTO user_media_items
                    (user_id, media_item_id, category_name, added_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![user_id, media_item_id, category, now_rfc3339()],
            )
            .map_err(|error| match error {
                rusqlite::Error::SqliteFailure(code, _)
                    if code.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    CoreError::Conflict("That item is already in your library.".into())
                }
                other => CoreError::Database(other),
            })?;
            Ok(conn.last_insert_rowid())
        })?;

        self.library_item_by_id(user_id, id)?.ok_or_else(|| {
            CoreError::Message("The item was added but could not be read back.".into())
        })
    }

    pub fn remove_user_media(&self, user_id: i64, id: i64) -> CoreResult<()> {
        self.with_connection(|conn| {
            let changed = conn.execute(
                "DELETE FROM user_media_items WHERE id = ?1 AND user_id = ?2",
                params![id, user_id],
            )?;
            if changed == 0 {
                return Err(CoreError::NotFound("Library item not found.".into()));
            }
            Ok(())
        })
    }

    pub fn clear_watch_history(&self, user_id: i64, id: i64) -> CoreResult<()> {
        self.with_connection(|conn| {
            let transaction = conn.transaction()?;
            let changed = transaction.execute(
                "UPDATE user_media_items SET
                    watch_progress_seconds = 0,
                    duration_seconds = 0,
                    is_completed = 0,
                    last_watched = NULL,
                    current_episode_file_name = NULL
                 WHERE id = ?1 AND user_id = ?2",
                params![id, user_id],
            )?;
            if changed == 0 {
                return Err(CoreError::NotFound("Library item not found.".into()));
            }
            transaction.execute(
                "DELETE FROM episode_progress WHERE user_media_item_id = ?1",
                params![id],
            )?;
            transaction.commit()?;
            Ok(())
        })
    }

    pub fn set_autoplay(&self, user_id: i64, id: i64, enabled: bool) -> CoreResult<()> {
        self.with_connection(|conn| {
            let changed = conn.execute(
                "UPDATE user_media_items SET autoplay_enabled = ?3 WHERE id = ?1 AND user_id = ?2",
                params![id, user_id, enabled as i64],
            )?;
            if changed == 0 {
                return Err(CoreError::NotFound("Library item not found.".into()));
            }
            Ok(())
        })
    }

    // ------------------------------------------------------------- playback

    pub fn user_media_for_identifier(
        &self,
        user_id: i64,
        identifier: &str,
    ) -> CoreResult<Option<UserMediaRow>> {
        self.with_connection(|conn| {
            Ok(conn
                .query_row(
                    r#"
                    SELECT umi.id, umi.media_item_id, umi.category_name,
                           umi.watch_progress_seconds, umi.duration_seconds,
                           umi.current_episode_file_name, umi.last_watched,
                           umi.is_completed, umi.autoplay_enabled
                    FROM user_media_items umi
                    JOIN media_items mi ON mi.id = umi.media_item_id
                    WHERE umi.user_id = ?1 AND mi.identifier = ?2
                    "#,
                    params![user_id, identifier],
                    map_user_media,
                )
                .optional()?)
        })
    }

    pub fn episode_progress(&self, user_media_item_id: i64) -> CoreResult<Vec<EpisodeProgress>> {
        self.with_connection(|conn| {
            let mut statement = conn.prepare(
                "SELECT file_name, position_seconds, duration_seconds, completed
                 FROM episode_progress WHERE user_media_item_id = ?1
                 ORDER BY file_name",
            )?;
            let rows = statement
                .query_map(params![user_media_item_id], |row| {
                    Ok(EpisodeProgress {
                        file_name: row.get(0)?,
                        position_seconds: row.get(1)?,
                        duration_seconds: row.get(2)?,
                        completed: row.get::<_, i64>(3)? != 0,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
    }

    /// Upserts the position for one episode and mirrors it onto the library
    /// item so "Continue watching" can show the right episode.
    pub fn save_progress(
        &self,
        user_id: i64,
        identifier: &str,
        file_name: &str,
        position_seconds: i64,
        duration_seconds: i64,
        completed: bool,
    ) -> CoreResult<()> {
        let position_seconds = position_seconds.max(0);
        let duration_seconds = duration_seconds.max(0);
        let episode_completed = completed
            || (duration_seconds > 0 && position_seconds >= duration_seconds.saturating_sub(3));

        self.with_connection(|conn| {
            let transaction = conn.transaction()?;
            let media_item_id: i64 = transaction
                .query_row(
                    "SELECT id FROM media_items WHERE identifier = ?1",
                    params![identifier],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or_else(|| CoreError::NotFound("Media item not found.".into()))?;

            let user_media_item_id: i64 = match transaction
                .query_row(
                    "SELECT id FROM user_media_items WHERE user_id = ?1 AND media_item_id = ?2",
                    params![user_id, media_item_id],
                    |row| row.get(0),
                )
                .optional()?
            {
                Some(id) => id,
                None => {
                    transaction.execute(
                        "INSERT INTO user_media_items
                            (user_id, media_item_id, category_name, added_at)
                         VALUES (?1, ?2, 'Uncategorized', ?3)",
                        params![user_id, media_item_id, now_rfc3339()],
                    )?;
                    transaction.last_insert_rowid()
                }
            };

            let now = now_rfc3339();
            transaction.execute(
                "INSERT INTO episode_progress
                    (user_media_item_id, file_name, position_seconds, duration_seconds, completed, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(user_media_item_id, file_name) DO UPDATE SET
                    position_seconds = excluded.position_seconds,
                    duration_seconds = excluded.duration_seconds,
                    completed = excluded.completed,
                    updated_at = excluded.updated_at",
                params![
                    user_media_item_id,
                    file_name,
                    position_seconds,
                    duration_seconds,
                    episode_completed as i64,
                    now,
                ],
            )?;

            transaction.execute(
                "UPDATE user_media_items SET
                    watch_progress_seconds = ?3,
                    duration_seconds = ?4,
                    current_episode_file_name = ?5,
                    last_watched = ?6,
                    is_completed = 0
                 WHERE id = ?1 AND user_id = ?2",
                params![
                    user_media_item_id,
                    user_id,
                    position_seconds,
                    duration_seconds,
                    file_name,
                    now,
                ],
            )?;

            transaction.commit()?;
            Ok(())
        })
    }

    pub fn mark_completed(
        &self,
        user_id: i64,
        identifier: &str,
        file_name: &str,
        item_completed: bool,
    ) -> CoreResult<()> {
        self.with_connection(|conn| {
            let transaction = conn.transaction()?;
            let user_media_item_id: i64 = transaction
                .query_row(
                    r#"
                    SELECT umi.id
                    FROM user_media_items umi
                    JOIN media_items mi ON mi.id = umi.media_item_id
                    WHERE umi.user_id = ?1 AND mi.identifier = ?2
                    "#,
                    params![user_id, identifier],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or_else(|| CoreError::NotFound("Library item not found.".into()))?;

            let now = now_rfc3339();
            let duration: i64 = transaction
                .query_row(
                    "SELECT duration_seconds FROM episode_progress
                     WHERE user_media_item_id = ?1 AND file_name = ?2",
                    params![user_media_item_id, file_name],
                    |row| row.get(0),
                )
                .optional()?
                .unwrap_or_default();

            transaction.execute(
                "INSERT INTO episode_progress
                    (user_media_item_id, file_name, position_seconds, duration_seconds, completed, updated_at)
                 VALUES (?1, ?2, ?3, ?3, 1, ?4)
                 ON CONFLICT(user_media_item_id, file_name) DO UPDATE SET
                    position_seconds = CASE
                        WHEN episode_progress.duration_seconds > 0 THEN episode_progress.duration_seconds
                        ELSE excluded.position_seconds
                    END,
                    completed = 1,
                    updated_at = excluded.updated_at",
                params![user_media_item_id, file_name, duration, now],
            )?;

            transaction.execute(
                "UPDATE user_media_items SET
                    is_completed = ?3,
                    watch_progress_seconds = CASE WHEN ?3 = 1 THEN duration_seconds ELSE watch_progress_seconds END,
                    last_watched = ?4
                 WHERE id = ?1 AND user_id = ?2",
                params![user_media_item_id, user_id, item_completed as i64, now],
            )?;

            transaction.commit()?;
            Ok(())
        })
    }
}

fn map_media(row: &Row<'_>) -> rusqlite::Result<MediaItemRow> {
    Ok(MediaItemRow {
        id: row.get(0)?,
        identifier: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        thumbnail_url: row.get(4)?,
        media_type: row.get(5)?,
        tmdb_poster_url: row.get(6)?,
        tmdb_id: row.get(7)?,
        tmdb_media_type: row.get(8)?,
    })
}

fn map_library_item(row: &Row<'_>) -> rusqlite::Result<LibraryItem> {
    Ok(LibraryItem {
        id: row.get(0)?,
        identifier: row.get(1)?,
        title: row.get(2)?,
        media_type: row.get(3)?,
        thumbnail_url: row.get(4)?,
        tmdb_poster_url: row.get(5)?,
        category_name: row.get(6)?,
        watch_progress_seconds: row.get(7)?,
        duration_seconds: row.get(8)?,
        current_episode_file_name: row.get(9)?,
        last_watched: row.get(10)?,
        is_completed: row.get::<_, i64>(11)? != 0,
        added_at: row.get(12)?,
    })
}

fn map_user_media(row: &Row<'_>) -> rusqlite::Result<UserMediaRow> {
    Ok(UserMediaRow {
        id: row.get(0)?,
        media_item_id: row.get(1)?,
        category_name: row.get(2)?,
        watch_progress_seconds: row.get(3)?,
        duration_seconds: row.get(4)?,
        current_episode_file_name: row.get(5)?,
        last_watched: row.get(6)?,
        is_completed: row.get::<_, i64>(7)? != 0,
        autoplay_enabled: row.get::<_, i64>(8)? != 0,
    })
}

fn group_library(items: Vec<LibraryItem>) -> LibraryResponse {
    let mut categories: Vec<CategoryGroup> = Vec::new();
    for item in items {
        match categories
            .iter_mut()
            .find(|group| group.name.eq_ignore_ascii_case(&item.category_name))
        {
            Some(group) => group.items.push(item),
            None => categories.push(CategoryGroup {
                name: item.category_name.clone(),
                items: vec![item],
            }),
        }
    }

    categories.sort_by(|left, right| {
        if left.name == "Uncategorized" {
            return std::cmp::Ordering::Greater;
        }
        if right.name == "Uncategorized" {
            return std::cmp::Ordering::Less;
        }
        left.name.to_lowercase().cmp(&right.name.to_lowercase())
    });

    let category_names = categories.iter().map(|group| group.name.clone()).collect();
    let total_items = categories.iter().map(|group| group.items.len()).sum();

    LibraryResponse {
        categories,
        category_names,
        total_items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::hash_password;

    fn database() -> Database {
        Database::in_memory().unwrap()
    }

    fn seed_user(db: &Database) -> User {
        db.create_user(
            "person@example.com",
            Some("Person"),
            &hash_password("password123").unwrap(),
        )
        .unwrap()
    }

    fn seed_media(db: &Database) -> MediaItemRow {
        db.insert_media(&NewMediaItem {
            identifier: "test-item".into(),
            title: "Test Item".into(),
            description: Some("A test".into()),
            thumbnail_url: None,
            media_type: "movies".into(),
            tmdb_poster_url: None,
            tmdb_id: None,
            tmdb_media_type: None,
        })
        .unwrap()
    }

    #[test]
    fn creates_and_finds_users() {
        let db = database();
        let user = seed_user(&db);
        assert_eq!(db.user_count().unwrap(), 1);
        assert_eq!(db.user_by_id(user.id).unwrap().unwrap().email, user.email);
        assert!(db.create_user("PERSON@example.com", None, "hash").is_err());
    }

    #[test]
    fn library_groups_and_removes() {
        let db = database();
        let user = seed_user(&db);
        let media = seed_media(&db);
        let item = db.add_user_media(user.id, media.id, "Movies").unwrap();
        assert_eq!(item.category_name, "Movies");
        assert!(db.add_user_media(user.id, media.id, "Movies").is_err());

        let library = db.library_for_user(user.id).unwrap();
        assert_eq!(library.total_items, 1);
        assert_eq!(library.category_names, vec!["Movies"]);

        db.remove_user_media(user.id, item.id).unwrap();
        assert_eq!(db.library_for_user(user.id).unwrap().total_items, 0);
    }

    #[test]
    fn tracks_progress_per_episode() {
        let db = database();
        let user = seed_user(&db);
        let media = seed_media(&db);
        let item = db.add_user_media(user.id, media.id, "TV").unwrap();

        db.save_progress(user.id, &media.identifier, "S01E01.mp4", 120, 1400, false)
            .unwrap();
        db.save_progress(user.id, &media.identifier, "S01E02.mp4", 30, 1400, false)
            .unwrap();

        let progress = db.episode_progress(item.id).unwrap();
        assert_eq!(progress.len(), 2);
        let recent = db.continue_watching(user.id, 10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(
            recent[0].current_episode_file_name.as_deref(),
            Some("S01E02.mp4")
        );

        db.mark_completed(user.id, &media.identifier, "S01E02.mp4", true)
            .unwrap();
        assert!(db.continue_watching(user.id, 10).unwrap().is_empty());
    }

    fn new_session(token_hash: &str, user_id: i64, expires_at: i64) -> NewSession {
        NewSession {
            token_hash: token_hash.to_string(),
            user_id,
            expires_at,
            kind: "browser".into(),
            label: None,
            platform: None,
            scopes: vec!["*".into()],
            last_seen_at: 0,
        }
    }

    #[test]
    fn stores_expires_and_deletes_sessions() {
        let db = database();
        let user = seed_user(&db);

        db.create_session(new_session("token-hash", user.id, 200))
            .unwrap();
        assert_eq!(
            db.session_user("token-hash", 100).unwrap().unwrap().id,
            user.id
        );
        assert!(db.session_user("token-hash", 200).unwrap().is_none());

        db.delete_expired_sessions(200).unwrap();
        assert!(db.session_user("token-hash", 100).unwrap().is_none());

        db.create_session(new_session("other-hash", user.id, 300))
            .unwrap();
        db.delete_session("other-hash").unwrap();
        assert!(db.session_user("other-hash", 100).unwrap().is_none());
        assert_eq!(db.first_user_id().unwrap(), Some(user.id));
    }

    #[test]
    fn upgrades_existing_sessions_to_v3() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy.db");
        let now = chrono::Utc::now().timestamp();
        {
            let connection = Connection::open(&path).unwrap();
            connection.execute_batch(SCHEMA).unwrap();
            connection.execute_batch(SESSION_SCHEMA).unwrap();
            connection
                .execute(
                    "INSERT INTO users (email, display_name, password_hash, created_at)
                     VALUES ('legacy@example.com', 'Legacy', 'hash', ?1)",
                    params![now_rfc3339()],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO sessions (token_hash, user_id, expires_at, created_at)
                     VALUES ('legacy-token', 1, ?1, ?2)",
                    params![now + 3600, now_rfc3339()],
                )
                .unwrap();
        }

        let db = Database::open(&path).unwrap();
        let session = db.session_auth("legacy-token", now).unwrap().unwrap();
        assert_eq!(session.user.email, "legacy@example.com");
        assert_eq!(session.session.kind, "browser");
        assert_eq!(session.session.scopes, vec!["*".to_string()]);
        assert!(!session.session.public_id.is_empty());
        assert!(session.session.last_seen_at.is_some());
        assert!(db.list_sessions(1, now).unwrap()[0].public_id.len() == 16);
    }

    #[test]
    fn clears_history() {
        let db = database();
        let user = seed_user(&db);
        let media = seed_media(&db);
        let item = db.add_user_media(user.id, media.id, "TV").unwrap();
        db.save_progress(user.id, &media.identifier, "S01E01.mp4", 120, 1400, false)
            .unwrap();
        db.clear_watch_history(user.id, item.id).unwrap();
        assert!(db.episode_progress(item.id).unwrap().is_empty());
        assert_eq!(
            db.user_media_for_identifier(user.id, &media.identifier)
                .unwrap()
                .unwrap()
                .watch_progress_seconds,
            0
        );
    }
}
