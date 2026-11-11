pub mod config;
mod error;
mod routes;
mod session;
mod state;

pub use config::Config;
pub use state::{AppState, SharedState};

use axum::Router;
use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue, header};
use axum::middleware::{self, Next};
use axum::response::Response;
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

/// Revalidate entry points so a deploy never serves an old `index.html` or
/// service worker, while hashed build assets can be cached forever.
async fn cache_control(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    let mut response = next.run(request).await;

    if path.starts_with("/api/") {
        return response;
    }

    let value = if path.starts_with("/assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static(value));
    response
}

pub fn build_router(state: SharedState) -> Router {
    let index = state.config.static_dir.join("index.html");
    let static_files =
        ServeDir::new(&state.config.static_dir).not_found_service(ServeFile::new(index));

    Router::new()
        .nest("/api", routes::api_router(state))
        .fallback_service(static_files)
        .layer(middleware::from_fn(cache_control))
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
