use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};

/// Errors returned by the core library and surfaced to the Tauri frontend.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid data received: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("{0}")]
    Conflict(String),

    #[error("Invalid input: {0}")]
    Invalid(String),

    #[error("You need to sign in first.")]
    Unauthorized,

    #[error("{0}")]
    Message(String),
}

impl CoreError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Database(_) => "database",
            Self::Network(_) => "network",
            Self::Io(_) => "io",
            Self::Json(_) => "invalid_data",
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
            Self::Invalid(_) => "invalid",
            Self::Unauthorized => "unauthorized",
            Self::Message(_) => "error",
        }
    }
}

impl Serialize for CoreError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("CoreError", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

pub type CoreResult<T> = Result<T, CoreError>;
