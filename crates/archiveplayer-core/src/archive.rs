use crate::error::{CoreError, CoreResult};
use crate::models::{ArchiveMetadata, ArchiveSearchResponse, ArchiveSearchResult};
use crate::util::{download_url, natural_cmp, strip_html};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::time::Duration;

const RESULTS_PER_PAGE: i64 = 12;
const USER_AGENT: &str =
    "ArchivePlayer/0.1 (Tauri desktop; +https://github.com/JeppeKjeldal/ArchivePlayer)";

#[derive(Clone)]
pub struct ArchiveClient {
    http: reqwest::Client,
}

impl ArchiveClient {
    pub fn new() -> CoreResult<Self> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(12))
            .build()?;
        Ok(Self { http })
    }

    pub async fn search(&self, query: &str, page: i64) -> CoreResult<ArchiveSearchResponse> {
        let query = query.trim();
        let page = page.max(1);
        if query.is_empty() {
            return Ok(ArchiveSearchResponse {
                results: Vec::new(),
                has_more_results: false,
                current_page: page,
                total_results: 0,
            });
        }

        let search_query = format!("mediatype:(movies OR tv OR animation) AND ({query})");
        let page_string = page.to_string();
        let rows_string = RESULTS_PER_PAGE.to_string();

        let response = self
            .http
            .get("https://archive.org/advancedsearch.php")
            .query(&[
                ("q", search_query.as_str()),
                ("fl[]", "identifier"),
                ("fl[]", "title"),
                ("fl[]", "mediatype"),
                ("rows", rows_string.as_str()),
                ("page", page_string.as_str()),
                ("output", "json"),
            ])
            .send()
            .await?
            .error_for_status()
            .map_err(|error| CoreError::Message(format!("Archive.org search failed: {error}")))?;

        let value: Value = response.json().await?;
        let response_node = value.get("response").ok_or_else(|| {
            CoreError::Message("Archive.org returned an unexpected response.".into())
        })?;

        let total_results = response_node
            .get("numFound")
            .and_then(value_to_i64)
            .unwrap_or(0);

        let mut results = Vec::new();
        if let Some(docs) = response_node.get("docs").and_then(Value::as_array) {
            for doc in docs {
                let Some(identifier) = doc.get("identifier").and_then(value_to_string) else {
                    continue;
                };
                let title = doc
                    .get("title")
                    .and_then(value_to_string)
                    .filter(|title| !title.trim().is_empty())
                    .unwrap_or_else(|| identifier.clone());
                let media_type = doc
                    .get("mediatype")
                    .and_then(value_to_string)
                    .unwrap_or_else(|| "Unknown".into());

                results.push(ArchiveSearchResult {
                    thumbnail_url: format!("https://archive.org/services/img/{identifier}"),
                    identifier,
                    title,
                    media_type,
                });
            }
        }

        Ok(ArchiveSearchResponse {
            has_more_results: page * RESULTS_PER_PAGE < total_results,
            results,
            current_page: page,
            total_results,
        })
    }

    pub async fn metadata(&self, identifier: &str) -> CoreResult<ArchiveMetadata> {
        let response = self
            .http
            .get(format!("https://archive.org/metadata/{identifier}"))
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(CoreError::NotFound(format!(
                "Archive.org has no item called “{identifier}”."
            )));
        }

        let response = response.error_for_status().map_err(|error| {
            CoreError::Message(format!("Could not load the Archive.org item: {error}"))
        })?;

        let value: Value = response.json().await?;
        if value
            .get("is_dark")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return Err(CoreError::NotFound(
                "This Archive.org item is restricted or unavailable.".into(),
            ));
        }

        let metadata = value.get("metadata").ok_or_else(|| {
            CoreError::NotFound("Archive.org returned no metadata for this item.".into())
        })?;

        let title = metadata
            .get("title")
            .and_then(value_to_string)
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| identifier.to_string());

        let description = metadata
            .get("description")
            .and_then(value_to_string)
            .map(|description| strip_html(&description))
            .filter(|description| !description.is_empty());

        let media_type = metadata
            .get("mediatype")
            .and_then(value_to_string)
            .unwrap_or_else(|| "Unknown".into());

        let mut files = Vec::new();
        if let Some(file_nodes) = value.get("files").and_then(Value::as_array) {
            for file in file_nodes {
                let Some(name) = file.get("name").and_then(Value::as_str) else {
                    continue;
                };
                if is_video_file(name) {
                    files.push(name.to_string());
                }
            }
        }

        let files = best_video_files(files);

        Ok(ArchiveMetadata {
            identifier: identifier.to_string(),
            title,
            description,
            media_type,
            video_files: files,
        })
    }

    pub fn download_url(&self, identifier: &str, file_name: &str) -> String {
        download_url(identifier, file_name)
    }

    pub fn item_url(&self, identifier: &str) -> String {
        format!("https://archive.org/details/{identifier}")
    }
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => items.first().and_then(value_to_string),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn value_to_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

pub fn is_video_file(file_name: &str) -> bool {
    let extension = file_name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();
    matches!(
        extension.as_str(),
        "mp4" | "m4v" | "webm" | "mkv" | "avi" | "ogv"
    )
}

/// Collapse alternate encodings of the same video (for example an item that
/// ships both `movie.mp4` and `movie.ogv`) into the single most playable file.
fn best_video_files(files: Vec<String>) -> Vec<String> {
    let mut by_stem: BTreeMap<String, String> = BTreeMap::new();

    for file_name in files {
        let stem = file_name
            .rsplit_once('.')
            .map(|(stem, _)| stem.to_string())
            .unwrap_or_else(|| file_name.clone());

        match by_stem.get(&stem) {
            Some(existing) => {
                let rank = playability(&file_name);
                let existing_rank = playability(existing);
                if rank < existing_rank
                    || (rank == existing_rank
                        && natural_cmp(&file_name, existing) == Ordering::Less)
                {
                    by_stem.insert(stem, file_name);
                }
            }
            None => {
                by_stem.insert(stem, file_name);
            }
        }
    }

    let mut files: Vec<String> = by_stem.into_values().collect();
    files.sort_by(|left, right| natural_cmp(left, right));
    files
}

fn playability(file_name: &str) -> u8 {
    let extension = file_name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "mp4" | "m4v" | "webm" => 0,
        "mkv" | "ogv" => 1,
        "avi" => 2,
        _ => 3,
    }
}

#[allow(dead_code)]
fn compare_files(left: &str, right: &str) -> Ordering {
    playability(left)
        .cmp(&playability(right))
        .then_with(|| natural_cmp(left, right))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_video_extensions() {
        assert!(is_video_file("movie.mp4"));
        assert!(is_video_file("movie.MKV"));
        assert!(!is_video_file("movie.srt"));
        assert!(!is_video_file("movie"));
    }

    #[test]
    fn collapses_alternate_video_encodings() {
        let files = best_video_files(vec![
            "movie.ogv".into(),
            "movie.mp4".into(),
            "episode-01.webm".into(),
            "episode-02.mkv".into(),
        ]);
        assert_eq!(
            files,
            vec![
                "episode-01.webm".to_string(),
                "episode-02.mkv".to_string(),
                "movie.mp4".to_string()
            ]
        );
    }

    #[test]
    fn reads_archive_value_shapes() {
        assert_eq!(
            value_to_string(&serde_json::json!(["One", "Two"])).as_deref(),
            Some("One")
        );
        assert_eq!(value_to_i64(&serde_json::json!("42")), Some(42));
    }
}
