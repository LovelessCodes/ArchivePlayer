use crate::config::Config;
use crate::session;
use archiveplayer_core::models::User;
use archiveplayer_core::{CoreError, CoreResult, Service};
use axum_extra::extract::cookie::CookieJar;
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

    pub fn current_user(&self, jar: &CookieJar) -> CoreResult<Option<User>> {
        let Some(cookie) = jar.get(session::COOKIE_NAME) else {
            return Ok(None);
        };
        let token_hash = session::hash_token(cookie.value());
        self.service.db.session_user(&token_hash, self.now())
    }

    pub fn require_user(&self, jar: &CookieJar) -> CoreResult<User> {
        self.current_user(jar)?.ok_or(CoreError::Unauthorized)
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
}
