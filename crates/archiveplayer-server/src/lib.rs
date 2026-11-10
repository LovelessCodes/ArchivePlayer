pub mod config;
mod error;
mod routes;
mod session;
mod state;

pub use config::Config;
pub use state::{AppState, SharedState};

use axum::Router;
use axum::http::{HeaderName, HeaderValue};
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

const CONTENT_SECURITY_POLICY: &str = "default-src 'self'; \
    img-src 'self' data: blob: https://archive.org https://*.archive.org https://image.tmdb.org; \
    media-src 'self' blob: https://archive.org https://*.archive.org; \
    connect-src 'self'; \
    script-src 'self'; \
    style-src 'self' 'unsafe-inline'; \
    font-src 'self' data:; \
    base-uri 'self'; form-action 'self'; frame-ancestors 'none'";

pub fn build_router(state: SharedState) -> Router {
    let index = state.config.static_dir.join("index.html");
    let static_files =
        ServeDir::new(&state.config.static_dir).not_found_service(ServeFile::new(index));

    Router::new()
        .nest("/api", routes::api_router(state))
        .fallback_service(static_files)
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static(CONTENT_SECURITY_POLICY),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}

pub fn app_state(service: archiveplayer_core::Service, config: Config) -> SharedState {
    Arc::new(AppState::new(service, config))
}
