use crate::config::Config;
use crate::error::ApiError;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Bucket {
    Api,
    Auth,
    Device,
}

#[derive(Clone, Copy)]
struct Limit {
    capacity: f64,
    refill_per_second: f64,
}

impl Limit {
    fn new(per_minute: u32) -> Option<Self> {
        (per_minute > 0).then(|| Self {
            capacity: f64::from(per_minute),
            refill_per_second: f64::from(per_minute) / 60.0,
        })
    }
}

struct Entry {
    tokens: f64,
    updated: Instant,
}

struct Inner {
    entries: HashMap<(Bucket, String), Entry>,
    last_cleanup: Instant,
}

#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<Inner>>,
    api: Option<Limit>,
    auth: Option<Limit>,
    device: Option<Limit>,
    trust_proxy: bool,
}

impl RateLimiter {
    pub fn from_config(config: &Config) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                entries: HashMap::new(),
                last_cleanup: Instant::now(),
            })),
            api: Limit::new(config.rate_limit_per_minute),
            auth: Limit::new(config.auth_rate_limit_per_minute),
            device: Limit::new(config.device_rate_limit_per_minute),
            trust_proxy: config.trust_proxy,
        }
    }

    fn limit(&self, bucket: Bucket) -> Option<Limit> {
        match bucket {
            Bucket::Api => self.api,
            Bucket::Auth => self.auth,
            Bucket::Device => self.device,
        }
    }

    fn client_key(&self, request: &Request, headers: &HeaderMap) -> String {
        if self.trust_proxy
            && let Some(address) = forwarded_address(headers)
        {
            return address.to_string();
        }
        request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(address)| address.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// Returns the time to wait when the request is not allowed.
    fn take(&self, bucket: Bucket, key: String, now: Instant) -> Option<Duration> {
        let limit = self.limit(bucket)?;
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if now.duration_since(inner.last_cleanup) >= Duration::from_secs(60) {
            inner.last_cleanup = now;
            if inner.entries.len() > 2_048 {
                inner.entries.retain(|_, entry| {
                    now.duration_since(entry.updated) < Duration::from_secs(900)
                });
            }
        }

        let entry = inner.entries.entry((bucket, key)).or_insert_with(|| Entry {
            tokens: limit.capacity,
            updated: now,
        });

        let elapsed = now.saturating_duration_since(entry.updated).as_secs_f64();
        if elapsed > 0.0 {
            entry.tokens = (entry.tokens + elapsed * limit.refill_per_second).min(limit.capacity);
            entry.updated = now;
        }

        if entry.tokens >= 1.0 {
            entry.tokens -= 1.0;
            None
        } else {
            Some(Duration::from_secs_f64(
                (1.0 - entry.tokens) / limit.refill_per_second,
            ))
        }
    }
}

fn forwarded_address(headers: &HeaderMap) -> Option<IpAddr> {
    let forwarded = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(value) = forwarded
        && let Ok(address) = value.parse()
    {
        return Some(address);
    }
    headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .and_then(|value| value.parse().ok())
}

fn bucket_for(path: &str) -> Option<Bucket> {
    if path == "/api/auth/login" || path == "/api/auth/register" {
        Some(Bucket::Auth)
    } else if path.starts_with("/api/device/") {
        Some(Bucket::Device)
    } else if path.starts_with("/api/") {
        Some(Bucket::Api)
    } else {
        None
    }
}

pub async fn enforce(State(limiter): State<RateLimiter>, request: Request, next: Next) -> Response {
    let Some(bucket) = bucket_for(request.uri().path()) else {
        return next.run(request).await;
    };
    if limiter.limit(bucket).is_none() {
        return next.run(request).await;
    }

    let key = limiter.client_key(&request, request.headers());
    if let Some(wait) = limiter.take(bucket, key, Instant::now()) {
        let seconds = wait.as_secs_f64().ceil().max(1.0) as u64;
        let mut response = ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "Too many requests. Try again shortly.",
        )
        .into_response();
        if let Ok(value) = HeaderValue::from_str(&seconds.to_string()) {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        return response;
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_only_api_routes() {
        assert!(matches!(bucket_for("/api/health"), Some(Bucket::Api)));
        assert!(matches!(bucket_for("/api/auth/login"), Some(Bucket::Auth)));
        assert!(matches!(
            bucket_for("/api/device/token"),
            Some(Bucket::Device)
        ));
        assert!(bucket_for("/").is_none());
        assert!(bucket_for("/assets/app.js").is_none());
    }

    #[test]
    fn parses_forwarded_addresses() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.9, 10.0.0.1"),
        );
        assert_eq!(
            forwarded_address(&headers),
            Some("203.0.113.9".parse().unwrap())
        );

        headers.insert("x-real-ip", HeaderValue::from_static("198.51.100.4"));
        headers.remove("x-forwarded-for");
        assert_eq!(
            forwarded_address(&headers),
            Some("198.51.100.4".parse().unwrap())
        );
    }
}
