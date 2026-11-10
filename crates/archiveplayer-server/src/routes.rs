use crate::error::{ApiError, ApiResult};
use crate::session;
use crate::state::{AppState, SharedState};
use archiveplayer_core::auth::{hash_password, validate_email, verify_password};
use archiveplayer_core::models::{
    AppInfo, ArchiveSearchResponse, AuthStatus, CategorySuggestion, LibraryItem, LibraryResponse,
    MediaDetails, RuntimeMode, SettingsInfo, User,
};
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, patch, post, put};
use axum::{Json, Router};
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};

pub fn api_router(state: SharedState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/auth/status", get(auth_status))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/library", get(get_library).post(add_to_library))
        .route("/library/continue-watching", get(get_continue_watching))
        .route("/library/{id}", delete(remove_from_library))
        .route("/library/{id}/history", delete(clear_watch_history))
        .route("/library/{id}/autoplay", patch(set_autoplay))
        .route("/search", get(search_archive))
        .route(
            "/category-suggestions/{identifier}",
            get(category_suggestions),
        )
        .route("/playback/{identifier}", get(get_playback_details))
        .route("/playback/{identifier}/progress", put(save_progress))
        .route("/playback/{identifier}/completed", post(mark_completed))
        .route("/settings", get(get_settings))
        .route("/settings/tmdb", put(save_tmdb_key).delete(clear_tmdb_key))
        .route("/settings/tmdb/test", post(test_tmdb_key))
        .route("/app-info", get(get_app_info))
        .fallback(api_not_found)
        .with_state(state)
}

async fn api_not_found() -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        "not_found",
        "API endpoint not found.",
    )
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
    version: &'static str,
}

async fn health(State(state): State<SharedState>) -> ApiResult<Json<Health>> {
    state.service.db.user_count()?;
    Ok(Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    }))
}

// ------------------------------------------------------------------- auth

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthRequest {
    email: String,
    password: String,
    display_name: Option<String>,
    remember: Option<bool>,
}

async fn auth_status(
    State(state): State<SharedState>,
    jar: CookieJar,
) -> ApiResult<Json<AuthStatus>> {
    let has_users = state.service.db.user_count()? > 0;
    let current_user = state.current_user(&jar)?;
    Ok(Json(AuthStatus {
        has_users,
        current_user,
        last_email: None,
        registration_open: state.registration_open(has_users),
        runtime: RuntimeMode::Server,
    }))
}

async fn register(
    State(state): State<SharedState>,
    jar: CookieJar,
    payload: Result<Json<AuthRequest>, JsonRejection>,
) -> ApiResult<(CookieJar, Json<User>)> {
    let Json(request) = payload.map_err(ApiError::from)?;
    let email = request.email.trim().to_lowercase();
    validate_email(&email)?;
    let display_name = request
        .display_name
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty());
    let password_hash = hash_password(&request.password)?;
    let remember = request.remember.unwrap_or(true);

    let _guard = state.registration_lock.lock().await;
    let has_users = state.service.db.user_count()? > 0;
    if has_users && !state.config.allow_registration {
        return Err(ApiError::forbidden(
            "Registration is closed on this server. Ask the server owner for an account.",
        ));
    }

    let user = state
        .service
        .db
        .create_user(&email, display_name.as_deref(), &password_hash)?;
    state.service.db.delete_expired_sessions(state.now())?;

    let token = session::new_token();
    state.service.db.create_session(
        &session::hash_token(&token),
        user.id,
        state.session_expiry(),
    )?;
    let jar = jar.add(session::build_cookie(token, remember, &state.config));
    Ok((jar, Json(user)))
}

async fn login(
    State(state): State<SharedState>,
    jar: CookieJar,
    payload: Result<Json<AuthRequest>, JsonRejection>,
) -> ApiResult<(CookieJar, Json<User>)> {
    let Json(request) = payload.map_err(ApiError::from)?;
    let email = request.email.trim().to_lowercase();
    let Some(auth_user) = state.service.db.find_user_by_email(&email)? else {
        return Err(ApiError::invalid("Invalid email or password."));
    };
    if !verify_password(&auth_user.password_hash, &request.password) {
        return Err(ApiError::invalid("Invalid email or password."));
    }

    if let Some(cookie) = jar.get(session::COOKIE_NAME) {
        state
            .service
            .db
            .delete_session(&session::hash_token(cookie.value()))?;
    }
    state.service.db.delete_expired_sessions(state.now())?;
    let token = session::new_token();
    state.service.db.create_session(
        &session::hash_token(&token),
        auth_user.user.id,
        state.session_expiry(),
    )?;
    let jar = jar.add(session::build_cookie(
        token,
        request.remember.unwrap_or(true),
        &state.config,
    ));
    Ok((jar, Json(auth_user.user)))
}

async fn logout(
    State(state): State<SharedState>,
    jar: CookieJar,
) -> ApiResult<(CookieJar, StatusCode)> {
    if let Some(cookie) = jar.get(session::COOKIE_NAME) {
        state
            .service
            .db
            .delete_session(&session::hash_token(cookie.value()))?;
    }
    Ok((
        jar.remove(session::removal_cookie()),
        StatusCode::NO_CONTENT,
    ))
}

// ---------------------------------------------------------------- library

async fn get_library(
    State(state): State<SharedState>,
    jar: CookieJar,
) -> ApiResult<Json<LibraryResponse>> {
    let user = state.require_user(&jar)?;
    Ok(Json(state.service.db.library_for_user(user.id)?))
}

#[derive(Deserialize)]
struct ContinueQuery {
    limit: Option<i64>,
}

async fn get_continue_watching(
    State(state): State<SharedState>,
    jar: CookieJar,
    Query(query): Query<ContinueQuery>,
) -> ApiResult<Json<Vec<LibraryItem>>> {
    let user = state.require_user(&jar)?;
    let limit = query.limit.unwrap_or(10).clamp(1, 50);
    Ok(Json(state.service.db.continue_watching(user.id, limit)?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddLibraryRequest {
    identifier: String,
    category_name: Option<String>,
}

async fn add_to_library(
    State(state): State<SharedState>,
    jar: CookieJar,
    payload: Result<Json<AddLibraryRequest>, JsonRejection>,
) -> ApiResult<Json<LibraryItem>> {
    let user = state.require_user(&jar)?;
    let Json(request) = payload.map_err(ApiError::from)?;
    let item = state
        .service
        .add_to_library(
            user.id,
            &request.identifier,
            request.category_name.as_deref().unwrap_or("Uncategorized"),
        )
        .await?;
    Ok(Json(item))
}

async fn remove_from_library(
    State(state): State<SharedState>,
    jar: CookieJar,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    let user = state.require_user(&jar)?;
    state.service.db.remove_user_media(user.id, id)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn clear_watch_history(
    State(state): State<SharedState>,
    jar: CookieJar,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    let user = state.require_user(&jar)?;
    state.service.db.clear_watch_history(user.id, id)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct AutoplayRequest {
    enabled: bool,
}

async fn set_autoplay(
    State(state): State<SharedState>,
    jar: CookieJar,
    Path(id): Path<i64>,
    payload: Result<Json<AutoplayRequest>, JsonRejection>,
) -> ApiResult<StatusCode> {
    let user = state.require_user(&jar)?;
    let Json(request) = payload.map_err(ApiError::from)?;
    state
        .service
        .db
        .set_autoplay(user.id, id, request.enabled)?;
    Ok(StatusCode::NO_CONTENT)
}

// ----------------------------------------------------------------- search

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    page: Option<i64>,
}

async fn search_archive(
    State(state): State<SharedState>,
    jar: CookieJar,
    Query(query): Query<SearchQuery>,
) -> ApiResult<Json<ArchiveSearchResponse>> {
    state.require_user(&jar)?;
    let term = query.q.trim();
    if term.is_empty() {
        return Err(ApiError::invalid("Enter a search term."));
    }
    Ok(Json(
        state
            .service
            .archive
            .search(term, query.page.unwrap_or(1).max(1))
            .await?,
    ))
}

async fn category_suggestions(
    State(state): State<SharedState>,
    jar: CookieJar,
    Path(identifier): Path<String>,
) -> ApiResult<Json<Vec<CategorySuggestion>>> {
    state.require_user(&jar)?;
    Ok(Json(state.service.category_suggestions(&identifier).await?))
}

// --------------------------------------------------------------- playback

async fn get_playback_details(
    State(state): State<SharedState>,
    jar: CookieJar,
    Path(identifier): Path<String>,
) -> ApiResult<Json<MediaDetails>> {
    let user = state.require_user(&jar)?;
    Ok(Json(
        state.service.playback_details(user.id, &identifier).await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProgressRequest {
    file_name: String,
    position_seconds: i64,
    duration_seconds: i64,
}

async fn save_progress(
    State(state): State<SharedState>,
    jar: CookieJar,
    Path(identifier): Path<String>,
    payload: Result<Json<ProgressRequest>, JsonRejection>,
) -> ApiResult<StatusCode> {
    let user = state.require_user(&jar)?;
    let Json(request) = payload.map_err(ApiError::from)?;
    state.service.db.save_progress(
        user.id,
        &identifier,
        &request.file_name,
        request.position_seconds,
        request.duration_seconds,
        false,
    )?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompletedRequest {
    file_name: String,
    item_completed: bool,
}

async fn mark_completed(
    State(state): State<SharedState>,
    jar: CookieJar,
    Path(identifier): Path<String>,
    payload: Result<Json<CompletedRequest>, JsonRejection>,
) -> ApiResult<StatusCode> {
    let user = state.require_user(&jar)?;
    let Json(request) = payload.map_err(ApiError::from)?;
    state.service.db.mark_completed(
        user.id,
        &identifier,
        &request.file_name,
        request.item_completed,
    )?;
    Ok(StatusCode::NO_CONTENT)
}

// --------------------------------------------------------------- settings

async fn get_settings(
    State(state): State<SharedState>,
    jar: CookieJar,
) -> ApiResult<Json<SettingsInfo>> {
    let user = state.require_user(&jar)?;
    let is_admin = state.is_admin(user.id)?;
    let mut info =
        state
            .service
            .settings_info(env!("CARGO_PKG_VERSION"), RuntimeMode::Server, is_admin);
    info.last_email = None;
    if !is_admin {
        info.tmdb_api_key = None;
    }
    Ok(Json(info))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveTmdbRequest {
    api_key: Option<String>,
}

async fn save_tmdb_key(
    State(state): State<SharedState>,
    jar: CookieJar,
    payload: Result<Json<SaveTmdbRequest>, JsonRejection>,
) -> ApiResult<StatusCode> {
    let user = state.require_user(&jar)?;
    require_admin(&state, user.id)?;
    let Json(request) = payload.map_err(ApiError::from)?;
    let key = request
        .api_key
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty());
    state.service.settings.set_tmdb_api_key(key)?;
    state.service.tmdb.clear_cache();
    Ok(StatusCode::NO_CONTENT)
}

async fn clear_tmdb_key(State(state): State<SharedState>, jar: CookieJar) -> ApiResult<StatusCode> {
    let user = state.require_user(&jar)?;
    require_admin(&state, user.id)?;
    state.service.settings.clear_tmdb_api_key()?;
    state.service.tmdb.clear_cache();
    Ok(StatusCode::NO_CONTENT)
}

async fn test_tmdb_key(State(state): State<SharedState>, jar: CookieJar) -> ApiResult<StatusCode> {
    let user = state.require_user(&jar)?;
    require_admin(&state, user.id)?;
    state.service.tmdb.validate_key().await?;
    Ok(StatusCode::NO_CONTENT)
}

fn require_admin(state: &AppState, user_id: i64) -> ApiResult<()> {
    if state.is_admin(user_id)? {
        Ok(())
    } else {
        Err(ApiError::forbidden(
            "Only the server owner can change this setting.",
        ))
    }
}

async fn get_app_info(
    State(state): State<SharedState>,
    jar: CookieJar,
) -> ApiResult<Json<AppInfo>> {
    state.require_user(&jar)?;
    Ok(Json(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        data_dir: state.service.data_dir.display().to_string(),
        settings_path: state.service.settings.path().display().to_string(),
        database_path: state.service.database_path.display().to_string(),
        runtime: RuntimeMode::Server,
    }))
}
