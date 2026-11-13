//! Playback-file selection.
//!
//! Archive.org items often carry several encodings of the same video. The
//! browser and Android clients can hand most of them to their own player; a
//! native webOS client has to pick a container the television's media pipeline
//! can actually open. This module keeps that policy on the server so every
//! client gets the same answer and can display a useful error when there is no
//! compatible file.

use crate::models::{PlaybackPlan, VideoFileUrl};
use std::cmp::Reverse;

pub const PLATFORM_WEBOS: &str = "webos";

pub fn normalize_platform(platform: Option<&str>) -> String {
    match platform.map(str::trim).filter(|value| !value.is_empty()) {
        None => "generic".into(),
        Some(value) => match value.to_ascii_lowercase().as_str() {
            "webos" | "lgwebos" | "lg-webos" | "lg" => PLATFORM_WEBOS.into(),
            "androidtv" | "android-tv" | "android" => "androidtv".into(),
            "web" | "browser" => "web".into(),
            other => other.to_string(),
        },
    }
}

pub fn extension(file_name: &str) -> Option<String> {
    file_name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .filter(|extension| !extension.is_empty())
}

pub fn container(file_name: &str) -> Option<String> {
    extension(file_name)
}

pub fn mime_type_for_file(file_name: &str) -> Option<&'static str> {
    match extension(file_name)?.as_str() {
        "mp4" | "m4v" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        "mkv" => Some("video/x-matroska"),
        "ogv" | "ogg" => Some("video/ogg"),
        "avi" => Some("video/x-msvideo"),
        "mov" => Some("video/quicktime"),
        "flv" => Some("video/x-flv"),
        "ts" => Some("video/mp2t"),
        "m3u8" => Some("application/vnd.apple.mpegurl"),
        _ => None,
    }
}

/// Best-effort codec label. Archive.org's `format` field is not a strict MIME
/// type, so this recognises the labels its derivatives actually use.
pub fn infer_video_codec(format: Option<&str>, file_name: &str) -> Option<&'static str> {
    let format = format.unwrap_or_default().to_ascii_lowercase();
    let ext = extension(file_name).unwrap_or_default();

    if format.contains("h.264") || format.contains("h264") || format.contains("avc") {
        return Some("h264");
    }
    if format.contains("h.265") || format.contains("h265") || format.contains("hevc") {
        return Some("hevc");
    }
    if format.contains("av1") {
        return Some("av1");
    }
    if format.contains("vp9") {
        return Some("vp9");
    }
    if format.contains("vp8") {
        return Some("vp8");
    }
    if format.contains("theora") {
        return Some("theora");
    }
    if format.contains("mpeg-2") || format.contains("mpeg2") {
        return Some("mpeg2");
    }
    if format.contains("mpeg-4") || format.contains("mpeg4") {
        return Some("mpeg4");
    }

    match ext.as_str() {
        "ogv" => Some("theora"),
        "webm" => None,
        _ => None,
    }
}

/// webOS URI playback is conservative: MP4/M4V, and not a codec we know the
/// television's media pipeline will refuse. Unknown MP4 codecs are allowed
/// because many Archive.org items simply do not publish a useful format label.
fn webos_compatible(file: &VideoFileUrl) -> bool {
    let ext = extension(&file.file_name).unwrap_or_default();
    if !matches!(ext.as_str(), "mp4" | "m4v") {
        return false;
    }
    !matches!(
        file.video_codec.as_deref(),
        Some("hevc" | "av1" | "vp9" | "vp8" | "theora" | "mpeg2")
    )
}

fn webos_reason(file: &VideoFileUrl) -> Option<String> {
    let ext = extension(&file.file_name).unwrap_or_default();
    if !matches!(ext.as_str(), "mp4" | "m4v") {
        return Some(format!(
            "This item only offers a .{ext} file; webOS direct playback needs an MP4/M4V file."
        ));
    }
    match file.video_codec.as_deref() {
        Some(codec @ ("hevc" | "av1" | "vp9" | "vp8" | "theora" | "mpeg2")) => Some(format!(
            "This MP4 is encoded as {codec}; this client only guarantees H.264 playback."
        )),
        _ => None,
    }
}

fn extension_rank(file_name: &str) -> u8 {
    match extension(file_name).as_deref() {
        Some("mp4" | "m4v") => 0,
        Some("webm") => 1,
        Some("mkv") => 2,
        Some("ogv" | "ogg") => 3,
        Some("avi") => 4,
        _ => 5,
    }
}

fn codec_rank(codec: Option<&str>) -> u8 {
    match codec {
        Some("h264") => 0,
        None => 1,
        Some("mpeg4") => 2,
        Some("hevc") => 3,
        Some("av1" | "vp9" | "vp8") => 4,
        Some("theora" | "mpeg2") => 5,
        _ => 6,
    }
}

fn resolution(file: &VideoFileUrl) -> i64 {
    file.width
        .zip(file.height)
        .map(|(width, height)| width.saturating_mul(height))
        .unwrap_or_default()
}

fn rank(file: &VideoFileUrl, platform: &str) -> (u8, u8, u8, Reverse<i64>, Reverse<u64>) {
    let compatibility = if platform == PLATFORM_WEBOS && !webos_compatible(file) {
        1
    } else {
        0
    };
    (
        compatibility,
        extension_rank(&file.file_name),
        codec_rank(file.video_codec.as_deref()),
        Reverse(resolution(file)),
        Reverse(file.size_bytes.unwrap_or_default()),
    )
}

/// Select one file for `platform`, honoring an explicit `file` request.
/// The request wins even when it is incompatible, so the client can show the
/// server's explanation instead of silently playing a different episode/quality.
pub fn build_playback_plan(
    files: &[VideoFileUrl],
    platform: Option<&str>,
    requested_file: Option<&str>,
) -> Option<PlaybackPlan> {
    let platform = normalize_platform(platform);
    let selected = requested_file
        .and_then(|name| files.iter().find(|file| file.file_name == name))
        .or_else(|| files.iter().min_by_key(|file| rank(file, &platform)))?;

    let compatible = platform != PLATFORM_WEBOS || webos_compatible(selected);
    let reason = if compatible {
        None
    } else {
        webos_reason(selected)
    };

    Some(PlaybackPlan {
        platform,
        file_name: selected.file_name.clone(),
        url: selected.url.clone(),
        mime_type: selected
            .mime_type
            .clone()
            .or_else(|| mime_type_for_file(&selected.file_name).map(str::to_string)),
        format: selected.format.clone(),
        container: container(&selected.file_name),
        video_codec: selected.video_codec.clone(),
        size_bytes: selected.size_bytes,
        length_seconds: selected.length_seconds,
        width: selected.width,
        height: selected.height,
        compatible,
        reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, format: Option<&str>) -> VideoFileUrl {
        VideoFileUrl {
            file_name: name.into(),
            url: format!("https://archive.org/download/id/{name}"),
            format: format.map(str::to_string),
            mime_type: mime_type_for_file(name).map(str::to_string),
            size_bytes: None,
            length_seconds: None,
            width: None,
            height: None,
            video_codec: infer_video_codec(format, name).map(str::to_string),
        }
    }

    #[test]
    fn webos_prefers_h264_mp4() {
        let files = vec![
            file("movie.mkv", Some("Matroska")),
            file("movie.mp4", Some("MPEG4")),
            file("movie-h264.mp4", Some("h.264 IA")),
        ];
        let plan = build_playback_plan(&files, Some("webos"), None).unwrap();
        assert_eq!(plan.file_name, "movie-h264.mp4");
        assert_eq!(plan.video_codec.as_deref(), Some("h264"));
        assert_eq!(plan.mime_type.as_deref(), Some("video/mp4"));
        assert!(plan.compatible);
        assert!(plan.reason.is_none());
    }

    #[test]
    fn webos_reports_incompatible_when_only_mkv_exists() {
        let files = vec![file("movie.mkv", Some("h.264"))];
        let plan = build_playback_plan(&files, Some("webos"), None).unwrap();
        assert_eq!(plan.file_name, "movie.mkv");
        assert!(!plan.compatible);
        assert!(plan.reason.unwrap().contains("MP4/M4V"));
    }

    #[test]
    fn explicit_file_wins_even_if_incompatible() {
        let files = vec![
            file("movie.mp4", Some("h.264")),
            file("movie.mkv", Some("h.264")),
        ];
        let plan = build_playback_plan(&files, Some("webos"), Some("movie.mkv")).unwrap();
        assert_eq!(plan.file_name, "movie.mkv");
        assert!(!plan.compatible);
    }

    #[test]
    fn generic_platform_accepts_first_best_file() {
        let files = vec![
            file("movie.mkv", Some("h.264")),
            file("movie.mp4", Some("h.264")),
        ];
        let plan = build_playback_plan(&files, None, None).unwrap();
        assert_eq!(plan.platform, "generic");
        assert_eq!(plan.file_name, "movie.mp4");
        assert!(plan.compatible);
    }

    #[test]
    fn recognizes_container_and_codec_labels() {
        assert_eq!(mime_type_for_file("a.MP4"), Some("video/mp4"));
        assert_eq!(mime_type_for_file("a.mkv"), Some("video/x-matroska"));
        assert_eq!(infer_video_codec(Some("h.264 IA"), "a.mp4"), Some("h264"));
        assert_eq!(infer_video_codec(Some("HEVC"), "a.mp4"), Some("hevc"));
        assert_eq!(
            infer_video_codec(Some("Ogg Video"), "a.ogv"),
            Some("theora")
        );
        assert_eq!(infer_video_codec(None, "a.mp4"), None);
    }
}
