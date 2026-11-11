use crate::error::ApiError;
use crate::state::SharedState;
use archiveplayer_core::db::AuthSession;
use archiveplayer_core::models::User;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

pub const SCOPE_LIBRARY_READ: &str = "library:read";
pub const SCOPE_LIBRARY_WRITE: &str = "library:write";
pub const SCOPE_PROGRESS_WRITE: &str = "progress:write";
pub const SCOPE_SETTINGS_MANAGE: &str = "settings:manage";

pub const DEVICE_DEFAULT_SCOPES: &[&str] = &[SCOPE_LIBRARY_READ, SCOPE_PROGRESS_WRITE];
pub const ALLOWED_DEVICE_SCOPES: &[&str] = &[
    SCOPE_LIBRARY_READ,
    SCOPE_LIBRARY_WRITE,
    SCOPE_PROGRESS_WRITE,
];

/// An authenticated browser or paired device. Handlers get the user plus the
/// session metadata so they can enforce scopes and revoke the right session.
pub struct AuthContext {
    pub user: User,
    pub session: AuthSession,
}

impl AuthContext {
    pub fn has_scope(&self, scope: &str) -> bool {
        self.session
            .session
            .scopes
            .iter()
            .any(|granted| granted == "*" || granted == scope)
    }

    pub fn require_scope(&self, scope: &str) -> Result<(), ApiError> {
        if self.has_scope(scope) {
            Ok(())
        } else {
            Err(ApiError::forbidden(format!(
                "This device was not granted the {scope} permission."
            )))
        }
    }

    pub fn require_browser(&self) -> Result<(), ApiError> {
        if self.session.session.kind == "browser" {
            Ok(())
        } else {
            Err(ApiError::forbidden(
                "Approve devices from a signed-in browser, not from a paired device.",
            ))
        }
    }
}

impl FromRequestParts<SharedState> for AuthContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &SharedState,
    ) -> Result<Self, Self::Rejection> {
        let session = state.require_session(&parts.headers)?;
        let user = session.user.clone();
        Ok(Self { user, session })
    }
}
