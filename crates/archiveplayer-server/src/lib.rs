mod auth;
pub mod config;
pub mod discovery;
mod error;
mod rate_limit;
mod routes;
mod session;
mod state;

pub const API_VERSION: u32 = 1;

pub use config::Config;
pub use state::{AppState, SharedState};

use axum::Router;
use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue, Method, header};
use axum::middleware::{self, Next};
use axum::response::Response;
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};
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
    let cors_origins = state.config.cors_origins.clone();
    let index = state.config.static_dir.join("index.html");
    let static_files =
        ServeDir::new(&state.config.static_dir).not_found_service(ServeFile::new(index));
    let limiter = rate_limit::RateLimiter::from_config(&state.config);

    let mut router = Router::new()
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
        .layer(TraceLayer::new_for_http());

    router = router.layer(middleware::from_fn_with_state(limiter, rate_limit::enforce));

    if !cors_origins.is_empty() {
        let origins: Vec<HeaderValue> = cors_origins
            .iter()
            .filter_map(|origin| HeaderValue::from_str(origin).ok())
            .collect();
        router = router.layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                ])
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
                .expose_headers([header::RETRY_AFTER]),
        );
    }

    router
}

pub fn app_state(service: archiveplayer_core::Service, config: Config) -> SharedState {
    Arc::new(AppState::new(service, config))
}
