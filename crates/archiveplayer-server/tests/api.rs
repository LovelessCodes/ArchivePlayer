use archiveplayer_core::Service;
use archiveplayer_server::{Config, SharedState, app_state, build_router};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

struct TestApp {
    router: Router,
    _dir: TempDir,
    state: SharedState,
}

fn test_app() -> TestApp {
    test_app_with(|_| {})
}

fn test_app_with(configure: impl FnOnce(&mut Config)) -> TestApp {
    let dir = tempfile::tempdir().unwrap();
    let static_dir = dir.path().join("static");
    std::fs::create_dir_all(&static_dir).unwrap();
    std::fs::write(
        static_dir.join("index.html"),
        "<!doctype html><title>ArchivePlayer test</title>",
    )
    .unwrap();
    std::fs::create_dir_all(static_dir.join("assets")).unwrap();
    std::fs::write(static_dir.join("assets/app-abc123.js"), "console.log(1);").unwrap();

    let mut config = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        data_dir: dir.path().join("data"),
        static_dir,
        secure_cookies: false,
        allow_registration: false,
        session_days: 30,
        device_session_days: 180,
        public_url: None,
        cors_origins: vec![],
        mdns: false,
        server_name: None,
        trust_proxy: false,
        rate_limit_per_minute: 0,
        auth_rate_limit_per_minute: 0,
        device_rate_limit_per_minute: 0,
    };
    configure(&mut config);
    let service = Service::new(config.data_dir.clone()).unwrap();
    let state = app_state(service, config);
    TestApp {
        router: build_router(state.clone()),
        _dir: dir,
        state,
    }
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn request(method: &str, uri: &str, body: Option<Value>, cookie: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    builder
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .unwrap()
}

fn bearer_request(method: &str, uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

fn session_cookie(response: &axum::response::Response) -> String {
    response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

async fn register(app: &TestApp, email: &str) -> (StatusCode, String) {
    let response = app
        .router
        .clone()
        .oneshot(request(
            "POST",
            "/api/auth/register",
            Some(json!({
                "email": email,
                "password": "password123",
                "displayName": "Tester",
                "remember": true
            })),
            None,
        ))
        .await
        .unwrap();
    let status = response.status();
    let cookie = if status == StatusCode::OK {
        session_cookie(&response)
    } else {
        String::new()
    };
    (status, cookie)
}

#[tokio::test]
async fn health_is_public() {
    let app = test_app();
    let response = app
        .router
        .clone()
        .oneshot(request("GET", "/api/health", None, None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CACHE_CONTROL)
            .unwrap()
            .to_str()
            .unwrap(),
        "no-store"
    );
    let body = body_json(response).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["apiVersion"], 1);
}

#[tokio::test]
async fn auth_rate_limit_returns_retry_after() {
    let app = test_app_with(|config| {
        config.trust_proxy = true;
        config.auth_rate_limit_per_minute = 1;
    });
    let request = |address: &'static str| {
        Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-forwarded-for", address)
            .body(Body::from("{}"))
            .unwrap()
    };

    let response = app
        .router
        .clone()
        .oneshot(request("203.0.113.9"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = app
        .router
        .clone()
        .oneshot(request("203.0.113.9"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(
        response
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .is_some_and(|seconds| seconds >= 1)
    );
    assert_eq!(body_json(response).await["code"], "rate_limited");

    // A different client address has its own bucket.
    let response = app
        .router
        .clone()
        .oneshot(request("203.0.113.10"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn library_requires_a_session() {
    let app = test_app();
    let response = app
        .router
        .clone()
        .oneshot(request("GET", "/api/library", None, None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(response).await["code"], "unauthorized");
}

#[tokio::test]
async fn registration_creates_one_owner_and_stops() {
    let app = test_app();

    let (status, cookie) = register(&app, "owner@example.com").await;
    assert_eq!(status, StatusCode::OK);

    let response = app
        .router
        .clone()
        .oneshot(request("GET", "/api/auth/status", None, Some(&cookie)))
        .await
        .unwrap();
    let body = body_json(response).await;
    assert_eq!(body["currentUser"]["email"], "owner@example.com");
    assert_eq!(body["registrationOpen"], false);
    assert_eq!(body["runtime"], "server");

    let (second_status, _) = register(&app, "someone@example.com").await;
    assert_eq!(second_status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn login_logout_round_trip() {
    let app = test_app();
    let (status, cookie) = register(&app, "owner@example.com").await;
    assert_eq!(status, StatusCode::OK);

    let response = app
        .router
        .clone()
        .oneshot(request("POST", "/api/auth/logout", None, Some(&cookie)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response = app
        .router
        .clone()
        .oneshot(request("GET", "/api/library", None, Some(&cookie)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = app
        .router
        .clone()
        .oneshot(request(
            "POST",
            "/api/auth/login",
            Some(json!({
                "email": "owner@example.com",
                "password": "password123"
            })),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let new_cookie = session_cookie(&response);
    assert_ne!(new_cookie, cookie);

    let response = app
        .router
        .clone()
        .oneshot(request("GET", "/api/library", None, Some(&new_cookie)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_json(response).await["totalItems"], 0);
}

#[tokio::test]
async fn serves_spa_and_json_404() {
    let app = test_app();
    let response = app
        .router
        .clone()
        .oneshot(request("GET", "/", None, None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response
            .headers()
            .get("content-security-policy")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("default-src 'self'")
    );
    assert_eq!(
        response
            .headers()
            .get(header::CACHE_CONTROL)
            .unwrap()
            .to_str()
            .unwrap(),
        "no-cache"
    );
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&body).contains("ArchivePlayer test"));

    let response = app
        .router
        .clone()
        .oneshot(request("GET", "/api/nope", None, None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(body_json(response).await["code"], "not_found");
}

#[tokio::test]
async fn first_user_owns_server_settings() {
    let app = test_app();
    let (_, owner_cookie) = register(&app, "owner@example.com").await;

    let response = app
        .router
        .clone()
        .oneshot(request("GET", "/api/settings", None, Some(&owner_cookie)))
        .await
        .unwrap();
    let body = body_json(response).await;
    assert_eq!(body["canManageSettings"], true);
    assert_eq!(body["runtime"], "server");
    assert_eq!(
        body["dataDir"],
        app.state.service.data_dir.display().to_string()
    );
}

#[tokio::test]
async fn hashed_assets_are_cached_forever() {
    let app = test_app();
    let response = app
        .router
        .clone()
        .oneshot(request("GET", "/assets/app-abc123.js", None, None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CACHE_CONTROL)
            .unwrap()
            .to_str()
            .unwrap(),
        "public, max-age=31536000, immutable"
    );
}

#[tokio::test]
async fn device_pairing_grants_scoped_bearer_token() {
    let app = test_app();
    let (status, cookie) = register(&app, "owner@example.com").await;
    assert_eq!(status, StatusCode::OK);

    let response = app
        .router
        .clone()
        .oneshot(request(
            "POST",
            "/api/device/start",
            Some(json!({
                "clientName": "Untrusted TV",
                "scopes": ["settings:manage"]
            })),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body_json(response).await["code"], "invalid");

    let response = app
        .router
        .clone()
        .oneshot(request(
            "POST",
            "/api/device/start",
            Some(json!({
                "clientName": "Living room TV",
                "platform": "webos",
                "scopes": ["library:read", "progress:write"]
            })),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let start = body_json(response).await;
    let device_code = start["deviceCode"].as_str().unwrap().to_string();
    let user_code = start["userCode"].as_str().unwrap().to_string();
    assert_eq!(start["interval"], 5);

    let response = app
        .router
        .clone()
        .oneshot(request(
            "POST",
            "/api/device/token",
            Some(json!({ "deviceCode": device_code })),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body_json(response).await["code"], "authorization_pending");

    let response = app
        .router
        .clone()
        .oneshot(request(
            "POST",
            "/api/device/lookup",
            Some(json!({ "userCode": user_code })),
            Some(&cookie),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let lookup = body_json(response).await;
    assert_eq!(lookup["clientName"], "Living room TV");
    assert_eq!(lookup["scopes"][0], "library:read");

    let response = app
        .router
        .clone()
        .oneshot(request(
            "POST",
            "/api/device/approve",
            Some(json!({ "userCode": user_code })),
            Some(&cookie),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // The first poll after approval can either redeem immediately or be
    // rate-limited by the interval from the pending poll above.
    let mut token_body = Value::Null;
    for attempt in 0..2 {
        let response = app
            .router
            .clone()
            .oneshot(request(
                "POST",
                "/api/device/token",
                Some(json!({ "deviceCode": device_code })),
                None,
            ))
            .await
            .unwrap();
        let status = response.status();
        token_body = body_json(response).await;
        if status == StatusCode::OK {
            break;
        }
        assert_eq!(
            (status, token_body["code"].as_str()),
            (StatusCode::TOO_MANY_REQUESTS, Some("slow_down")),
            "unexpected token response: {token_body}"
        );
        assert_eq!(attempt, 0, "still rate-limited after waiting");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
    let token = token_body["token"].as_str().unwrap().to_string();
    assert_eq!(token.len(), 64);

    let response = app
        .router
        .clone()
        .oneshot(bearer_request("GET", "/api/library", &token))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .router
        .clone()
        .oneshot(bearer_request("GET", "/api/settings", &token))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = app
        .router
        .clone()
        .oneshot(request("GET", "/api/devices", None, Some(&cookie)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let devices = body_json(response).await;
    let device = devices
        .as_array()
        .unwrap()
        .iter()
        .find(|session| session["kind"] == "device")
        .expect("paired device is listed");
    assert_eq!(device["label"], "Living room TV");
    let device_id = device["id"].as_str().unwrap().to_string();

    let response = app
        .router
        .clone()
        .oneshot(request(
            "DELETE",
            &format!("/api/devices/{device_id}"),
            None,
            Some(&cookie),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response = app
        .router
        .clone()
        .oneshot(bearer_request("GET", "/api/library", &token))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
