use crate::error::{CoreError, CoreResult};
use crate::models::{
    ArchiveMetadata, ArchiveSearchResponse, ArchiveSearchResult, ArchiveVideoFile,
};
use crate::playback;
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
                if let Some(file) = archive_video_file(file) {
                    files.push(file);
                }
            }
        }

        let files = best_video_files(files);
        let video_files = files.iter().map(|file| file.name.clone()).collect();

        Ok(ArchiveMetadata {
            identifier: identifier.to_string(),
            title,
            description,
            media_type,
            video_files,
            video_file_details: files,
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

fn value_to_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

fn value_to_seconds(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => parse_length(text),
        _ => None,
    }
}

fn parse_length(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if !text.contains(':') {
        return text.parse().ok();
    }
    let parts: Vec<&str> = text.split(':').collect();
    let values: Vec<f64> = parts
        .iter()
        .map(|part| part.trim().parse().ok())
        .collect::<Option<_>>()?;
    if values.iter().any(|value| *value < 0.0) {
        return None;
    }
    match values.as_slice() {
        [minutes, seconds] => Some(minutes * 60.0 + seconds),
        [hours, minutes, seconds] => Some(hours * 3600.0 + minutes * 60.0 + seconds),
        _ => None,
    }
}

fn archive_video_file(file: &Value) -> Option<ArchiveVideoFile> {
    let name = file.get("name").and_then(Value::as_str)?;
    if !is_video_file(name) {
        return None;
    }
    Some(ArchiveVideoFile {
        name: name.to_string(),
        format: file.get("format").and_then(value_to_string),
        size_bytes: file.get("size").and_then(value_to_u64),
        length_seconds: file.get("length").and_then(value_to_seconds),
        width: file.get("width").and_then(value_to_i64),
        height: file.get("height").and_then(value_to_i64),
    })
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
fn best_video_files(files: Vec<ArchiveVideoFile>) -> Vec<ArchiveVideoFile> {
    let mut by_stem: BTreeMap<String, ArchiveVideoFile> = BTreeMap::new();

    for file in files {
        let stem = file
            .name
            .rsplit_once('.')
            .map(|(stem, _)| stem.to_string())
            .unwrap_or_else(|| file.name.clone());

        match by_stem.get(&stem) {
            Some(existing) => {
                let rank = playability(&file);
                let existing_rank = playability(existing);
                if rank < existing_rank
                    || (rank == existing_rank
                        && natural_cmp(&file.name, &existing.name) == Ordering::Less)
                {
                    by_stem.insert(stem, file);
                }
            }
            None => {
                by_stem.insert(stem, file);
            }
        }
    }

    let mut files: Vec<ArchiveVideoFile> = by_stem.into_values().collect();
    files.sort_by(|left, right| natural_cmp(&left.name, &right.name));
    files
}

fn playability(file: &ArchiveVideoFile) -> (u8, u8) {
    let extension_rank = match playback::extension(&file.name).as_deref() {
        Some("mp4" | "m4v") => 0,
        Some("webm") => 0,
        Some("mkv" | "ogv") => 1,
        Some("avi") => 2,
        _ => 3,
    };
    let codec_rank = match playback::infer_video_codec(file.format.as_deref(), &file.name) {
        Some("h264") => 0,
        None => 1,
        Some("mpeg4") => 2,
        Some("hevc") => 3,
        Some("av1" | "vp9" | "vp8") => 4,
        Some("theora" | "mpeg2") => 5,
        _ => 6,
    };
    (extension_rank, codec_rank)
}

#[allow(dead_code)]
fn compare_files(left: &ArchiveVideoFile, right: &ArchiveVideoFile) -> Ordering {
    playability(left)
        .cmp(&playability(right))
        .then_with(|| natural_cmp(&left.name, &right.name))
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

    fn video(name: &str, format: Option<&str>) -> ArchiveVideoFile {
        ArchiveVideoFile {
            name: name.into(),
            format: format.map(str::to_string),
            size_bytes: None,
            length_seconds: None,
            width: None,
            height: None,
        }
    }

    #[test]
    fn collapses_alternate_video_encodings() {
        let files = best_video_files(vec![
            video("movie.ogv", Some("Ogg Video")),
            video("movie.mp4", Some("MPEG4")),
            video("episode-01.webm", None),
            video("episode-02.mkv", None),
        ]);
        assert_eq!(
            files.into_iter().map(|file| file.name).collect::<Vec<_>>(),
            vec![
                "episode-01.webm".to_string(),
                "episode-02.mkv".to_string(),
                "movie.mp4".to_string()
            ]
        );
    }

    #[test]
    fn keeps_the_better_codec_for_the_same_stem() {
        let files = best_video_files(vec![
            video("movie.mp4", Some("MPEG4")),
            video("movie.ia.mp4", Some("MPEG4")),
            video("movie.ia.mp4", Some("h.264 IA")),
        ]);
        let movie = files
            .iter()
            .find(|file| file.name == "movie.ia.mp4")
            .unwrap();
        assert_eq!(movie.format.as_deref(), Some("h.264 IA"));
    }

    #[test]
    fn reads_archive_value_shapes() {
        assert_eq!(
            value_to_string(&serde_json::json!(["One", "Two"])).as_deref(),
            Some("One")
        );
        assert_eq!(value_to_i64(&serde_json::json!("42")), Some(42));
        assert_eq!(value_to_u64(&serde_json::json!("1234")), Some(1234));
        assert_eq!(
            value_to_seconds(&serde_json::json!("7199.99")),
            Some(7199.99)
        );
        assert_eq!(
            value_to_seconds(&serde_json::json!("1:02:03")),
            Some(3723.0)
        );
        assert_eq!(value_to_seconds(&serde_json::json!("02:03")), Some(123.0));
    }

    #[test]
    fn reads_richer_file_metadata() {
        let file = archive_video_file(&serde_json::json!({
            "name": "movie.mp4",
            "format": "h.264 IA",
            "size": "1854950726",
            "length": "7199.99",
            "width": "640",
            "height": 360
        }))
        .unwrap();
        assert_eq!(file.format.as_deref(), Some("h.264 IA"));
        assert_eq!(file.size_bytes, Some(1_854_950_726));
        assert_eq!(file.length_seconds, Some(7199.99));
        assert_eq!(file.width, Some(640));
        assert_eq!(file.height, Some(360));
    }
}
