use archiveplayer_core::CoreError;
use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: String,
    message: String,
    detail: Option<String>,
}

#[derive(Serialize)]
struct ErrorBody {
    code: String,
    message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &str, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.to_string(),
            message: message.into(),
            detail: None,
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "invalid", message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", message)
    }
}

impl From<CoreError> for ApiError {
    fn from(error: CoreError) -> Self {
        let (status, code, message) = match &error {
            CoreError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "You need to sign in first.".to_string(),
            ),
            CoreError::Forbidden(message) => (StatusCode::FORBIDDEN, "forbidden", message.clone()),
            CoreError::Invalid(message) => (StatusCode::BAD_REQUEST, "invalid", message.clone()),
            CoreError::NotFound(message) => (StatusCode::NOT_FOUND, "not_found", message.clone()),
            CoreError::Conflict(message) => (StatusCode::CONFLICT, "conflict", message.clone()),
            CoreError::Network(_) => (StatusCode::BAD_GATEWAY, "network", error.to_string()),
            CoreError::Database(_)
            | CoreError::Io(_)
            | CoreError::Json(_)
            | CoreError::Message(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "Something went wrong on the server.".to_string(),
            ),
        };

        Self {
            status,
            code: code.to_string(),
            message,
            detail: (status == StatusCode::INTERNAL_SERVER_ERROR).then(|| error.to_string()),
        }
    }
}

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid".into(),
            message: "The request body was not valid JSON.".into(),
            detail: Some(rejection.body_text()),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if let Some(detail) = &self.detail {
            if self.status.is_server_error() {
                tracing::error!("request failed: {detail}");
            } else {
                tracing::debug!("request rejected: {detail}");
            }
        }

        (
            self.status,
            Json(ErrorBody {
                code: self.code,
                message: self.message,
            }),
        )
            .into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
