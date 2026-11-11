use crate::config::Config;
use crate::session;
use archiveplayer_core::db::AuthSession;
use archiveplayer_core::models::User;
use archiveplayer_core::{CoreError, CoreResult, Service};
use axum::http::HeaderMap;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub service: Service,
    pub config: Config,
    pub registration_lock: Mutex<()>,
}

pub type SharedState = Arc<AppState>;

impl AppState {
    pub fn new(service: Service, config: Config) -> Self {
        Self {
            service,
            config,
            registration_lock: Mutex::new(()),
        }
    }

    pub fn now(&self) -> i64 {
        Utc::now().timestamp()
    }

    pub fn current_session(&self, headers: &HeaderMap) -> CoreResult<Option<AuthSession>> {
        let Some(token) = session::request_token(headers) else {
            return Ok(None);
        };
        let token_hash = session::hash_token(&token);
        let now = self.now();
        let Some(auth) = self.service.db.session_auth(&token_hash, now)? else {
            return Ok(None);
        };
        if auth
            .session
            .last_seen_at
            .is_none_or(|last_seen| last_seen < now - 60)
        {
            self.service.db.touch_session(&token_hash, now)?;
        }
        Ok(Some(auth))
    }

    pub fn current_user(&self, headers: &HeaderMap) -> CoreResult<Option<User>> {
        Ok(self.current_session(headers)?.map(|auth| auth.user))
    }

    pub fn require_session(&self, headers: &HeaderMap) -> CoreResult<AuthSession> {
        self.current_session(headers)?
            .ok_or(CoreError::Unauthorized)
    }

    pub fn require_user(&self, headers: &HeaderMap) -> CoreResult<User> {
        Ok(self.require_session(headers)?.user)
    }

    pub fn registration_open(&self, has_users: bool) -> bool {
        self.config.allow_registration || !has_users
    }

    pub fn is_admin(&self, user_id: i64) -> CoreResult<bool> {
        Ok(self.service.db.first_user_id()? == Some(user_id))
    }

    pub fn session_expiry(&self) -> i64 {
        self.now() + self.config.session_days * 24 * 60 * 60
    }

    pub fn device_session_expiry(&self) -> i64 {
        self.now() + self.config.device_session_days * 24 * 60 * 60
    }
}
