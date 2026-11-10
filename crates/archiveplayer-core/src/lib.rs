pub mod archive;
pub mod auth;
pub mod db;
pub mod episodes;
pub mod error;
pub mod models;
pub mod service;
pub mod settings;
pub mod title_cleaner;
pub mod tmdb;
pub mod util;

pub use archive::ArchiveClient;
pub use auth::{hash_password, validate_email, validate_password, verify_password};
pub use db::Database;
pub use error::{CoreError, CoreResult};
pub use service::Service;
pub use settings::{AppSettings, SettingsStore};
pub use tmdb::TmdbClient;
