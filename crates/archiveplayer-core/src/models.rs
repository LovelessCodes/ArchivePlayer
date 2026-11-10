use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i64,
    pub email: String,
    pub display_name: Option<String>,
    pub created_at: String,
}

impl User {
    pub fn label(&self) -> String {
        self.display_name
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(&self.email)
            .to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub has_users: bool,
    pub current_user: Option<User>,
    pub last_email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItem {
    pub id: i64,
    pub identifier: String,
    pub title: String,
    pub media_type: String,
    pub thumbnail_url: Option<String>,
    pub tmdb_poster_url: Option<String>,
    pub category_name: String,
    pub watch_progress_seconds: i64,
    pub duration_seconds: i64,
    pub current_episode_file_name: Option<String>,
    pub last_watched: Option<String>,
    pub is_completed: bool,
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryGroup {
    pub name: String,
    pub items: Vec<LibraryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryResponse {
    pub categories: Vec<CategoryGroup>,
    pub category_names: Vec<String>,
    pub total_items: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveSearchResult {
    pub identifier: String,
    pub title: String,
    pub media_type: String,
    pub thumbnail_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveSearchResponse {
    pub results: Vec<ArchiveSearchResult>,
    pub has_more_results: bool,
    pub current_page: i64,
    pub total_results: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategorySuggestion {
    pub name: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
    pub name: Option<String>,
    pub overview: Option<String>,
    pub still_url: Option<String>,
    pub file_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeProgress {
    pub file_name: String,
    pub position_seconds: i64,
    pub duration_seconds: i64,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFileUrl {
    pub file_name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaDetails {
    pub identifier: String,
    pub title: String,
    pub description: Option<String>,
    pub media_type: String,
    pub thumbnail_url: Option<String>,
    pub tmdb_poster_url: Option<String>,
    pub tmdb_id: Option<i64>,
    pub tmdb_media_type: Option<String>,
    pub video_files: Vec<String>,
    pub video_file_urls: Vec<VideoFileUrl>,
    pub episodes: Vec<Episode>,
    pub watch_progress_seconds: i64,
    pub duration_seconds: i64,
    pub current_episode_file_name: Option<String>,
    pub last_watched: Option<String>,
    pub is_completed: bool,
    pub autoplay_enabled: bool,
    pub library_item_id: Option<i64>,
    pub episode_progress: Vec<EpisodeProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveMetadata {
    pub identifier: String,
    pub title: String,
    pub description: Option<String>,
    pub media_type: String,
    pub video_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsInfo {
    pub tmdb_api_key: Option<String>,
    pub tmdb_configured: bool,
    pub last_email: Option<String>,
    pub data_dir: String,
    pub settings_path: String,
    pub database_path: String,
    pub app_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub settings_path: String,
    pub database_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadedMedia {
    pub identifier: String,
    pub title: String,
    pub media_type: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub tmdb_poster_url: Option<String>,
    pub tmdb_id: Option<i64>,
    pub tmdb_media_type: Option<String>,
}
