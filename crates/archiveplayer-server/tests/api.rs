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
    let dir = tempfile::tempdir().unwrap();
    let static_dir = dir.path().join("static");
    std::fs::create_dir_all(&static_dir).unwrap();
    std::fs::write(
        static_dir.join("index.html"),
        "<!doctype html><title>ArchivePlayer test</title>",
    )
    .unwrap();

    let config = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        data_dir: dir.path().join("data"),
        static_dir,
        secure_cookies: false,
        allow_registration: false,
        session_days: 30,
    };
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
    let body = body_json(response).await;
    assert_eq!(body["status"], "ok");
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
