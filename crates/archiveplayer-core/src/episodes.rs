use crate::models::Episode;
use crate::util::humanize_file_name;
use regex::Regex;
use std::sync::OnceLock;

fn episode_regexes() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            // S01E02, S1 E2, s01.e02
            Regex::new(r"(?i)(?:^|[^a-z0-9])s(\d{1,2})[\s._-]*e(\d{1,3})(?:[^0-9]|$)")
                .expect("valid regex"),
            // 1x02
            Regex::new(r"(?i)(?:^|[^0-9])(\d{1,2})x(\d{2,3})(?:[^0-9]|$)").expect("valid regex"),
            // Episode 2, Ep. 2
            Regex::new(r"(?i)(?:episode|ep)[\s._-]*(\d{1,3})(?:[^0-9]|$)").expect("valid regex"),
            // "Show E02"
            Regex::new(r"(?i)(?:^|[^a-z])e(\d{1,3})(?:[^0-9]|$)").expect("valid regex"),
            // "02 - title"
            Regex::new(r"^(\d{1,3})[\s._-]").expect("valid regex"),
        ]
    })
}

/// Parses season/episode numbers out of a file name when present.
pub fn parse_episode_numbers(file_name: &str) -> Option<(i32, i32)> {
    let file_name = file_name.rsplit('/').next().unwrap_or(file_name);
    let regexes = episode_regexes();

    if let Some(captures) = regexes[0].captures(file_name)
        && let (Some(season), Some(episode)) = (captures.get(1), captures.get(2))
    {
        return Some((
            season.as_str().parse().ok()?,
            episode.as_str().parse().ok()?,
        ));
    }

    if let Some(captures) = regexes[1].captures(file_name)
        && let (Some(season), Some(episode)) = (captures.get(1), captures.get(2))
    {
        return Some((
            season.as_str().parse().ok()?,
            episode.as_str().parse().ok()?,
        ));
    }

    for regex in &regexes[2..] {
        if let Some(captures) = regex.captures(file_name)
            && let Some(episode) = captures.get(1)
        {
            return Some((1, episode.as_str().parse().ok()?));
        }
    }

    None
}

/// Builds a filename-derived episode list. Used when TMDB is not configured
/// or when no TMDB episode can be matched to a file.
pub fn parse_episodes_from_files(video_files: &[String]) -> Vec<Episode> {
    let parse_numbers = video_files.len() > 1;

    video_files
        .iter()
        .map(|file_name| {
            let numbers = if parse_numbers {
                parse_episode_numbers(file_name)
            } else {
                None
            };

            Episode {
                season_number: numbers.map(|(season, _)| season),
                episode_number: numbers.map(|(_, episode)| episode),
                name: Some(humanize_file_name(file_name)),
                overview: None,
                still_url: None,
                file_name: file_name.clone(),
            }
        })
        .collect()
}

/// Finds the video file that corresponds to a TMDB episode.
pub fn match_episode_to_file(
    season_number: i32,
    episode_number: i32,
    name: Option<&str>,
    video_files: &[String],
) -> Option<String> {
    for file_name in video_files {
        if parse_episode_numbers(file_name) == Some((season_number, episode_number)) {
            return Some(file_name.clone());
        }
    }

    // Some Archive.org uploads keep the episode title in the file name but
    // omit season/episode markers. Try a conservative title match.
    let normalized_name = name.map(normalize_for_match).unwrap_or_default();
    if normalized_name.len() >= 6 {
        for file_name in video_files {
            let normalized_file = normalize_for_match(file_name);
            if normalized_file.contains(&normalized_name) {
                return Some(file_name.clone());
            }
        }
    }

    None
}

fn normalize_for_match(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_episode_formats() {
        assert_eq!(parse_episode_numbers("Show.S01E05.720p.mkv"), Some((1, 5)));
        assert_eq!(parse_episode_numbers("Show - s2 e10.mp4"), Some((2, 10)));
        assert_eq!(parse_episode_numbers("Show 3x07.avi"), Some((3, 7)));
        assert_eq!(parse_episode_numbers("Show Episode 12.mp4"), Some((1, 12)));
        assert_eq!(parse_episode_numbers("Show E04.mp4"), Some((1, 4)));
        assert_eq!(parse_episode_numbers("12 - The Test.mp4"), Some((1, 12)));
        assert_eq!(parse_episode_numbers("A Movie (1999).mp4"), None);
    }

    #[test]
    fn matches_tmdb_episode_to_file() {
        let files = vec!["Show.S01E01.mp4".to_string(), "Show.S01E02.mp4".to_string()];
        assert_eq!(
            match_episode_to_file(1, 2, Some("The Second"), &files),
            Some("Show.S01E02.mp4".to_string())
        );
    }

    #[test]
    fn builds_fallback_episodes() {
        let files = vec!["Show.S01E02.mp4".to_string(), "Show.S01E01.mp4".to_string()];
        let episodes = parse_episodes_from_files(&files);
        assert_eq!(episodes.len(), 2);
        assert_eq!(episodes[0].season_number, Some(1));
        assert_eq!(episodes[0].episode_number, Some(2));
        assert!(episodes[0].name.as_deref().unwrap().contains("Show"));
    }
}
