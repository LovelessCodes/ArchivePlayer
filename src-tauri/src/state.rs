use archiveplayer_core::models::User;
use archiveplayer_core::{CoreError, CoreResult, Service};
use std::sync::Mutex;

/// Shared application state. The signed-in user is intentionally kept in
/// memory only, so closing the app signs the profile out.
pub struct AppState {
    pub service: Service,
    session: Mutex<Option<i64>>,
}

impl AppState {
    pub fn new(service: Service) -> Self {
        Self {
            service,
            session: Mutex::new(None),
        }
    }

    pub fn current_user_opt(&self) -> CoreResult<Option<User>> {
        let user_id = *self
            .session
            .lock()
            .map_err(|_| CoreError::Message("The session lock was poisoned.".into()))?;

        let Some(user_id) = user_id else {
            return Ok(None);
        };

        match self.service.db.user_by_id(user_id)? {
            Some(user) => Ok(Some(user)),
            None => {
                self.set_session(None)?;
                Ok(None)
            }
        }
    }

    pub fn current_user(&self) -> CoreResult<User> {
        self.current_user_opt()?.ok_or(CoreError::Unauthorized)
    }

    pub fn set_session(&self, user_id: Option<i64>) -> CoreResult<()> {
        let mut session = self
            .session
            .lock()
            .map_err(|_| CoreError::Message("The session lock was poisoned.".into()))?;
        *session = user_id;
        Ok(())
    }
}
