use crate::auth::{
    ALLOWED_DEVICE_SCOPES, AuthContext, DEVICE_DEFAULT_SCOPES, SCOPE_LIBRARY_READ,
    SCOPE_LIBRARY_WRITE, SCOPE_PROGRESS_WRITE, SCOPE_SETTINGS_MANAGE,
};
use crate::error::{ApiError, ApiResult};
use crate::session;
use crate::state::{AppState, SharedState};
use archiveplayer_core::auth::{hash_password, validate_email, verify_password};
use archiveplayer_core::db::{NewDeviceAuthorization, NewSession, SessionRecord};
use archiveplayer_core::models::{
    AppInfo, ArchiveSearchResponse, AuthStatus, CategorySuggestion, LibraryItem, LibraryResponse,
    MediaDetails, RuntimeMode, SettingsInfo, User,
};
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::routing::{delete, get, patch, post, put};
use axum::{Json, Router};
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use tower_http::set_header::SetResponseHeaderLayer;

pub fn api_router(state: SharedState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/auth/status", get(auth_status))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/device/start", post(device_start))
        .route("/device/token", post(device_token))
        .route("/device/lookup", post(device_lookup))
        .route("/device/approve", post(device_approve))
        .route("/devices", get(list_devices))
        .route("/devices/{id}", delete(revoke_device))
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
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
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
    headers: HeaderMap,
) -> ApiResult<Json<AuthStatus>> {
    let has_users = state.service.db.user_count()? > 0;
    let current_user = state.current_user(&headers)?;
    Ok(Json(AuthStatus {
        has_users,
        current_user,
        last_email: None,
        registration_open: state.registration_open(has_users),
        runtime: RuntimeMode::Server,
    }))
}

fn create_browser_session(
    state: &AppState,
    headers: &HeaderMap,
    user_id: i64,
) -> ApiResult<String> {
    let token = session::new_token();
    state.service.db.create_session(NewSession {
        token_hash: session::hash_token(&token),
        user_id,
        expires_at: state.session_expiry(),
        kind: "browser".into(),
        label: Some(session::browser_label(headers)),
        platform: Some("web".into()),
        scopes: vec!["*".into()],
        last_seen_at: state.now(),
    })?;
    Ok(token)
}

async fn register(
    State(state): State<SharedState>,
    headers: HeaderMap,
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

    let token = create_browser_session(&state, &headers, user.id)?;
    let jar = jar.add(session::build_cookie(token, remember, &state.config));
    Ok((jar, Json(user)))
}

async fn login(
    State(state): State<SharedState>,
    headers: HeaderMap,
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
    let token = create_browser_session(&state, &headers, auth_user.user.id)?;
    let jar = jar.add(session::build_cookie(
        token,
        request.remember.unwrap_or(true),
        &state.config,
    ));
    Ok((jar, Json(auth_user.user)))
}

async fn logout(
    State(state): State<SharedState>,
    headers: HeaderMap,
    jar: CookieJar,
) -> ApiResult<(CookieJar, StatusCode)> {
    if let Some(token) = session::request_token(&headers) {
        state
            .service
            .db
            .delete_session(&session::hash_token(&token))?;
    }
    Ok((
        jar.remove(session::removal_cookie()),
        StatusCode::NO_CONTENT,
    ))
}

// ---------------------------------------------------------------- library

async fn get_library(
    State(state): State<SharedState>,
    auth: AuthContext,
) -> ApiResult<Json<LibraryResponse>> {
    auth.require_scope(SCOPE_LIBRARY_READ)?;
    let user = &auth.user;
    Ok(Json(state.service.db.library_for_user(user.id)?))
}

#[derive(Deserialize)]
struct ContinueQuery {
    limit: Option<i64>,
}

async fn get_continue_watching(
    State(state): State<SharedState>,
    auth: AuthContext,
    Query(query): Query<ContinueQuery>,
) -> ApiResult<Json<Vec<LibraryItem>>> {
    auth.require_scope(SCOPE_LIBRARY_READ)?;
    let user = &auth.user;
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
    auth: AuthContext,
    payload: Result<Json<AddLibraryRequest>, JsonRejection>,
) -> ApiResult<Json<LibraryItem>> {
    auth.require_scope(SCOPE_LIBRARY_WRITE)?;
    let user = &auth.user;
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
    auth: AuthContext,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    auth.require_scope(SCOPE_LIBRARY_WRITE)?;
    let user = &auth.user;
    state.service.db.remove_user_media(user.id, id)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn clear_watch_history(
    State(state): State<SharedState>,
    auth: AuthContext,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    auth.require_scope(SCOPE_LIBRARY_WRITE)?;
    let user = &auth.user;
    state.service.db.clear_watch_history(user.id, id)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct AutoplayRequest {
    enabled: bool,
}

async fn set_autoplay(
    State(state): State<SharedState>,
    auth: AuthContext,
    Path(id): Path<i64>,
    payload: Result<Json<AutoplayRequest>, JsonRejection>,
) -> ApiResult<StatusCode> {
    auth.require_scope(SCOPE_LIBRARY_WRITE)?;
    let user = &auth.user;
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
    auth: AuthContext,
    Query(query): Query<SearchQuery>,
) -> ApiResult<Json<ArchiveSearchResponse>> {
    auth.require_scope(SCOPE_LIBRARY_READ)?;
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
    auth: AuthContext,
    Path(identifier): Path<String>,
) -> ApiResult<Json<Vec<CategorySuggestion>>> {
    auth.require_scope(SCOPE_LIBRARY_READ)?;
    Ok(Json(state.service.category_suggestions(&identifier).await?))
}

// --------------------------------------------------------------- playback

async fn get_playback_details(
    State(state): State<SharedState>,
    auth: AuthContext,
    Path(identifier): Path<String>,
) -> ApiResult<Json<MediaDetails>> {
    auth.require_scope(SCOPE_LIBRARY_READ)?;
    let user = &auth.user;
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
    auth: AuthContext,
    Path(identifier): Path<String>,
    payload: Result<Json<ProgressRequest>, JsonRejection>,
) -> ApiResult<StatusCode> {
    auth.require_scope(SCOPE_PROGRESS_WRITE)?;
    let user = &auth.user;
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
    auth: AuthContext,
    Path(identifier): Path<String>,
    payload: Result<Json<CompletedRequest>, JsonRejection>,
) -> ApiResult<StatusCode> {
    auth.require_scope(SCOPE_PROGRESS_WRITE)?;
    let user = &auth.user;
    let Json(request) = payload.map_err(ApiError::from)?;
    state.service.db.mark_completed(
        user.id,
        &identifier,
        &request.file_name,
        request.item_completed,
    )?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------- devices

const DEVICE_CODE_LIFETIME_SECONDS: i64 = 600;
const DEVICE_POLL_INTERVAL_SECONDS: i64 = 5;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceStartRequest {
    client_name: Option<String>,
    platform: Option<String>,
    scopes: Option<Vec<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceStartResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: String,
    expires_in: i64,
    interval: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceTokenRequest {
    device_code: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceTokenResponse {
    token: String,
    user: User,
    expires_at: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserCodeRequest {
    user_code: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceLookupResponse {
    client_name: String,
    platform: String,
    scopes: Vec<String>,
    status: String,
    expires_in: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionView {
    id: String,
    kind: String,
    label: Option<String>,
    platform: Option<String>,
    scopes: Vec<String>,
    created_at: String,
    last_seen_at: Option<i64>,
    expires_at: i64,
    current: bool,
}

fn session_view(session: SessionRecord, current_id: &str) -> SessionView {
    SessionView {
        current: session.public_id == current_id,
        id: session.public_id,
        kind: session.kind,
        label: session.label,
        platform: session.platform,
        scopes: session.scopes,
        created_at: session.created_at,
        last_seen_at: session.last_seen_at,
        expires_at: session.expires_at,
    }
}

fn selected_device_scopes(requested: Option<Vec<String>>) -> ApiResult<Vec<String>> {
    let requested = requested.unwrap_or_else(|| {
        DEVICE_DEFAULT_SCOPES
            .iter()
            .map(|scope| (*scope).to_string())
            .collect()
    });
    let mut scopes = Vec::new();
    for scope in requested {
        let scope = scope.trim();
        if scope.is_empty() {
            continue;
        }
        if !ALLOWED_DEVICE_SCOPES.contains(&scope) {
            return Err(ApiError::invalid(format!(
                "The {scope} permission cannot be granted to a paired device."
            )));
        }
        if !scopes.iter().any(|existing| existing == scope) {
            scopes.push(scope.to_string());
        }
    }
    if scopes.is_empty() {
        return Err(ApiError::invalid(
            "A paired device needs at least one permission.",
        ));
    }
    Ok(scopes)
}

fn verification_base(state: &AppState, headers: &HeaderMap) -> String {
    if let Some(url) = &state.config.public_url {
        return url.trim_end_matches('/').to_string();
    }
    let fallback = state.config.bind.to_string();
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .unwrap_or(&fallback);
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .filter(|value| *value == "https")
        .unwrap_or("http");
    format!("{scheme}://{host}")
}

async fn device_start(
    State(state): State<SharedState>,
    headers: HeaderMap,
    payload: Result<Json<DeviceStartRequest>, JsonRejection>,
) -> ApiResult<Json<DeviceStartResponse>> {
    let Json(request) = payload.map_err(ApiError::from)?;
    let client_name = request
        .client_name
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "ArchivePlayer TV".to_string());
    let client_name = client_name.chars().take(80).collect::<String>();
    let platform = request
        .platform
        .map(|platform| platform.trim().to_string())
        .filter(|platform| !platform.is_empty())
        .unwrap_or_else(|| "tv".to_string());
    let platform = platform.chars().take(40).collect::<String>();
    let scopes = selected_device_scopes(request.scopes)?;
    let now = state.now();
    state.service.db.delete_expired_sessions(now)?;
    state.service.db.delete_expired_device_authorizations(now)?;

    let mut authorization = None;
    for _ in 0..5 {
        let device_code = session::new_token();
        let user_code = session::new_user_code();
        match state
            .service
            .db
            .create_device_authorization(NewDeviceAuthorization {
                device_code_hash: session::hash_token(&device_code),
                user_code_hash: session::hash_token(&user_code.replace('-', "")),
                client_name: client_name.clone(),
                platform: platform.clone(),
                scopes: scopes.clone(),
                expires_at: now + DEVICE_CODE_LIFETIME_SECONDS,
                interval_seconds: DEVICE_POLL_INTERVAL_SECONDS,
            }) {
            Ok(()) => {
                authorization = Some((device_code, user_code));
                break;
            }
            Err(archiveplayer_core::CoreError::Conflict(_)) => continue,
            Err(error) => return Err(error.into()),
        }
    }
    let Some((device_code, user_code)) = authorization else {
        return Err(ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "Could not create a pairing code. Try again.",
        ));
    };

    let base = verification_base(&state, &headers);
    let verification_uri = format!("{base}/#/activate");
    Ok(Json(DeviceStartResponse {
        device_code,
        user_code: user_code.clone(),
        verification_uri: verification_uri.clone(),
        verification_uri_complete: format!("{verification_uri}?code={user_code}"),
        expires_in: DEVICE_CODE_LIFETIME_SECONDS,
        interval: DEVICE_POLL_INTERVAL_SECONDS,
    }))
}

async fn device_token(
    State(state): State<SharedState>,
    payload: Result<Json<DeviceTokenRequest>, JsonRejection>,
) -> ApiResult<Json<DeviceTokenResponse>> {
    let Json(request) = payload.map_err(ApiError::from)?;
    let device_code = request.device_code.trim();
    if device_code.is_empty() {
        return Err(ApiError::invalid("Missing device code."));
    }
    let device_code_hash = session::hash_token(device_code);
    let now = state.now();
    let Some(authorization) = state
        .service
        .db
        .device_authorization_by_device_code(&device_code_hash)?
    else {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "This pairing request is no longer valid.",
        ));
    };
    if authorization.expires_at <= now {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "expired_token",
            "This pairing code has expired. Start again on the TV.",
        ));
    }

    match authorization.status.as_str() {
        "pending" => {
            if authorization
                .last_polled_at
                .is_some_and(|last| last + authorization.interval_seconds > now)
            {
                return Err(ApiError::new(
                    StatusCode::TOO_MANY_REQUESTS,
                    "slow_down",
                    "Wait a few seconds before checking again.",
                ));
            }
            state.service.db.record_device_poll(authorization.id, now)?;
            Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "authorization_pending",
                "Waiting for approval.",
            ))
        }
        "denied" => Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "access_denied",
            "The pairing request was declined.",
        )),
        "approved" => {
            let token = session::new_token();
            let expires_at = state.device_session_expiry();
            let Some(user) = state.service.db.redeem_device_authorization(
                &device_code_hash,
                &session::hash_token(&token),
                expires_at,
                now,
            )?
            else {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "invalid_grant",
                    "This pairing request was already completed.",
                ));
            };
            Ok(Json(DeviceTokenResponse {
                token,
                user,
                expires_at,
            }))
        }
        _ => Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "This pairing request is no longer valid.",
        )),
    }
}

async fn device_lookup(
    State(state): State<SharedState>,
    auth: AuthContext,
    payload: Result<Json<UserCodeRequest>, JsonRejection>,
) -> ApiResult<Json<DeviceLookupResponse>> {
    auth.require_browser()?;
    let Json(request) = payload.map_err(ApiError::from)?;
    let Some(user_code) = session::normalize_user_code(&request.user_code) else {
        return Err(ApiError::invalid(
            "That code does not look right. Use the eight characters shown on the TV.",
        ));
    };
    let now = state.now();
    let Some(authorization) = state
        .service
        .db
        .device_authorization_by_user_code(&session::hash_token(&user_code))?
    else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "not_found",
            "No pending pairing request uses that code.",
        ));
    };
    if authorization.expires_at <= now {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "expired_token",
            "That pairing code has expired.",
        ));
    }
    Ok(Json(DeviceLookupResponse {
        client_name: authorization.client_name,
        platform: authorization.platform,
        scopes: authorization.scopes,
        status: authorization.status,
        expires_in: authorization.expires_at - now,
    }))
}

async fn device_approve(
    State(state): State<SharedState>,
    auth: AuthContext,
    payload: Result<Json<UserCodeRequest>, JsonRejection>,
) -> ApiResult<StatusCode> {
    auth.require_browser()?;
    let Json(request) = payload.map_err(ApiError::from)?;
    let Some(user_code) = session::normalize_user_code(&request.user_code) else {
        return Err(ApiError::invalid("That pairing code is not valid."));
    };
    let user_code_hash = session::hash_token(&user_code);
    let now = state.now();
    let Some(authorization) = state
        .service
        .db
        .device_authorization_by_user_code(&user_code_hash)?
    else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "not_found",
            "No pending pairing request uses that code.",
        ));
    };
    if authorization.expires_at <= now {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "expired_token",
            "That pairing code has expired.",
        ));
    }
    if state
        .service
        .db
        .set_device_authorization_status(&user_code_hash, auth.user.id, "approved", now)?
        .is_none()
    {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "conflict",
            "That pairing request was already handled.",
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn list_devices(
    State(state): State<SharedState>,
    auth: AuthContext,
) -> ApiResult<Json<Vec<SessionView>>> {
    auth.require_browser()?;
    let current_id = &auth.session.session.public_id;
    let sessions = state.service.db.list_sessions(auth.user.id, state.now())?;
    Ok(Json(
        sessions
            .into_iter()
            .map(|session| session_view(session, current_id))
            .collect(),
    ))
}

async fn revoke_device(
    State(state): State<SharedState>,
    auth: AuthContext,
    jar: CookieJar,
    Path(public_id): Path<String>,
) -> ApiResult<(CookieJar, StatusCode)> {
    auth.require_browser()?;
    let removed = state
        .service
        .db
        .delete_session_by_public_id(auth.user.id, &public_id)?;
    if !removed {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "not_found",
            "That session is already signed out.",
        ));
    }
    let jar = if public_id == auth.session.session.public_id {
        jar.add(session::removal_cookie())
    } else {
        jar
    };
    Ok((jar, StatusCode::NO_CONTENT))
}

// --------------------------------------------------------------- settings

async fn get_settings(
    State(state): State<SharedState>,
    auth: AuthContext,
) -> ApiResult<Json<SettingsInfo>> {
    auth.require_scope(SCOPE_SETTINGS_MANAGE)?;
    let user = &auth.user;
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
    auth: AuthContext,
    payload: Result<Json<SaveTmdbRequest>, JsonRejection>,
) -> ApiResult<StatusCode> {
    auth.require_scope(SCOPE_SETTINGS_MANAGE)?;
    let user = &auth.user;
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

async fn clear_tmdb_key(
    State(state): State<SharedState>,
    auth: AuthContext,
) -> ApiResult<StatusCode> {
    auth.require_scope(SCOPE_SETTINGS_MANAGE)?;
    let user = &auth.user;
    require_admin(&state, user.id)?;
    state.service.settings.clear_tmdb_api_key()?;
    state.service.tmdb.clear_cache();
    Ok(StatusCode::NO_CONTENT)
}

async fn test_tmdb_key(
    State(state): State<SharedState>,
    auth: AuthContext,
) -> ApiResult<StatusCode> {
    auth.require_scope(SCOPE_SETTINGS_MANAGE)?;
    let user = &auth.user;
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
    auth: AuthContext,
) -> ApiResult<Json<AppInfo>> {
    auth.require_scope(SCOPE_LIBRARY_READ)?;
    Ok(Json(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        data_dir: state.service.data_dir.display().to_string(),
        settings_path: state.service.settings.path().display().to_string(),
        database_path: state.service.database_path.display().to_string(),
        runtime: RuntimeMode::Server,
    }))
}
