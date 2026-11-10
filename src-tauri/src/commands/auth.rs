use crate::state::AppState;
use archiveplayer_core::auth::{hash_password, validate_email, verify_password};
use archiveplayer_core::models::{AuthStatus, User};
use archiveplayer_core::{CoreError, CoreResult};
use tauri::State;

#[tauri::command]
pub fn auth_status(state: State<'_, AppState>) -> CoreResult<AuthStatus> {
    Ok(AuthStatus {
        has_users: state.service.db.user_count()? > 0,
        current_user: state.current_user_opt()?,
        last_email: state.service.settings.get().last_email,
    })
}

#[tauri::command]
pub fn register(
    state: State<'_, AppState>,
    email: String,
    display_name: Option<String>,
    password: String,
    remember: Option<bool>,
) -> CoreResult<User> {
    let email = email.trim().to_lowercase();
    validate_email(&email)?;

    let display_name = display_name
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty());

    let password_hash = hash_password(&password)?;
    let user = state
        .service
        .db
        .create_user(&email, display_name.as_deref(), &password_hash)?;

    state.set_session(Some(user.id))?;
    state
        .service
        .settings
        .set_last_email(remember.unwrap_or(true).then(|| user.email.clone()))?;

    Ok(user)
}

#[tauri::command]
pub fn login(
    state: State<'_, AppState>,
    email: String,
    password: String,
    remember: Option<bool>,
) -> CoreResult<User> {
    let email = email.trim().to_lowercase();
    let Some(auth_user) = state.service.db.find_user_by_email(&email)? else {
        return Err(CoreError::Message("Invalid email or password.".into()));
    };

    if !verify_password(&auth_user.password_hash, &password) {
        return Err(CoreError::Message("Invalid email or password.".into()));
    }

    state.set_session(Some(auth_user.user.id))?;
    state.service.settings.set_last_email(
        remember
            .unwrap_or(true)
            .then(|| auth_user.user.email.clone()),
    )?;

    Ok(auth_user.user)
}

#[tauri::command]
pub fn logout(state: State<'_, AppState>) -> CoreResult<()> {
    state.set_session(None)
}
