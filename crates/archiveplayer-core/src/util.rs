use crate::error::{CoreError, CoreResult};
use regex::Regex;
use std::cmp::Ordering;
use std::sync::OnceLock;

pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// A small natural-order comparator so `Episode 2` sorts before `Episode 10`.
pub fn natural_cmp(left: &str, right: &str) -> Ordering {
    #[derive(PartialEq, Eq)]
    enum Chunk {
        Number(String),
        Text(String),
    }

    fn chunks(value: &str) -> Vec<Chunk> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut current_is_digit = None;

        for ch in value.chars() {
            let is_digit = ch.is_ascii_digit();
            if current_is_digit.is_some_and(|kind| kind != is_digit) {
                let text = std::mem::take(&mut current);
                result.push(if is_digit {
                    Chunk::Number(text)
                } else {
                    Chunk::Text(text.to_lowercase())
                });
            }
            current_is_digit = Some(is_digit);
            current.push(ch);
        }

        if !current.is_empty() {
            result.push(if current_is_digit == Some(true) {
                Chunk::Number(current)
            } else {
                Chunk::Text(current.to_lowercase())
            });
        }
        result
    }

    let left_chunks = chunks(left);
    let right_chunks = chunks(right);

    for (a, b) in left_chunks.iter().zip(right_chunks.iter()) {
        let ordering = match (a, b) {
            (Chunk::Number(a), Chunk::Number(b)) => {
                let a_trimmed = a.trim_start_matches('0');
                let b_trimmed = b.trim_start_matches('0');
                a_trimmed
                    .len()
                    .cmp(&b_trimmed.len())
                    .then_with(|| a_trimmed.cmp(b_trimmed))
                    .then_with(|| a.len().cmp(&b.len()))
            }
            (Chunk::Text(a), Chunk::Text(b)) => a.cmp(b),
            (Chunk::Number(_), Chunk::Text(_)) => Ordering::Less,
            (Chunk::Text(_), Chunk::Number(_)) => Ordering::Greater,
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }

    left_chunks.len().cmp(&right_chunks.len())
}

pub fn strip_html(value: &str) -> String {
    static TAG: OnceLock<Regex> = OnceLock::new();
    static WHITESPACE: OnceLock<Regex> = OnceLock::new();

    let tag = TAG.get_or_init(|| Regex::new(r"(?is)<[^>]*>").expect("valid regex"));
    let whitespace = WHITESPACE.get_or_init(|| Regex::new(r"\s+").expect("valid regex"));

    let without_tags = tag.replace_all(value, " ");
    let decoded = without_tags
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&ndash;", "–")
        .replace("&mdash;", "—");

    whitespace.replace_all(decoded.trim(), " ").to_string()
}

/// Accepts either a bare Archive.org identifier or a details URL.
pub fn parse_archive_identifier(input: &str) -> CoreResult<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Invalid(
            "Enter an Archive.org URL or identifier.".into(),
        ));
    }

    let identifier = if let Some((_, rest)) = trimmed.split_once("archive.org/details/") {
        rest.split(['?', '#'])
            .next()
            .unwrap_or(rest)
            .trim_end_matches('/')
    } else if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Err(CoreError::Invalid(
            "That does not look like an Archive.org details URL.".into(),
        ));
    } else {
        trimmed
            .split(['?', '#'])
            .next()
            .unwrap_or(trimmed)
            .trim_end_matches('/')
    };

    if identifier.is_empty()
        || !identifier
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~'))
    {
        return Err(CoreError::Invalid(
            "Archive.org identifiers may only contain letters, numbers, dots, dashes, underscores, and tildes.".into(),
        ));
    }

    Ok(identifier.to_string())
}

pub fn encode_path_segment(segment: &str) -> String {
    const UNRESERVED: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'.')
        .remove(b'_')
        .remove(b'~');
    percent_encoding::utf8_percent_encode(segment, UNRESERVED).to_string()
}

pub fn download_url(identifier: &str, file_name: &str) -> String {
    let encoded_identifier = encode_path_segment(identifier);
    let encoded_file = file_name
        .split('/')
        .map(encode_path_segment)
        .collect::<Vec<_>>()
        .join("/");
    format!("https://archive.org/download/{encoded_identifier}/{encoded_file}")
}

pub fn humanize_file_name(file_name: &str) -> String {
    let stem = file_name
        .rsplit('/')
        .next()
        .unwrap_or(file_name)
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(file_name);

    static SEPARATORS: OnceLock<Regex> = OnceLock::new();
    static WHITESPACE: OnceLock<Regex> = OnceLock::new();
    let separators = SEPARATORS.get_or_init(|| Regex::new(r"[._]+").expect("valid regex"));
    let whitespace = WHITESPACE.get_or_init(|| Regex::new(r"\s+").expect("valid regex"));

    let spaced = separators.replace_all(stem, " ");
    whitespace.replace_all(spaced.trim(), " ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_sorting() {
        assert_eq!(natural_cmp("Episode 2", "Episode 10"), Ordering::Less);
        assert_eq!(natural_cmp("s01e09", "s01e10"), Ordering::Less);
        assert_eq!(natural_cmp("B", "a"), Ordering::Greater);
    }

    #[test]
    fn parses_identifiers() {
        assert_eq!(
            parse_archive_identifier("https://archive.org/details/night_of_the_living_dead/")
                .unwrap(),
            "night_of_the_living_dead"
        );
        assert_eq!(
            parse_archive_identifier("https://archive.org/details/foo?start=1").unwrap(),
            "foo"
        );
        assert_eq!(parse_archive_identifier("foo-bar_1").unwrap(), "foo-bar_1");
        assert!(parse_archive_identifier("https://example.com/details/foo").is_err());
        assert!(parse_archive_identifier("bad identifier").is_err());
    }

    #[test]
    fn strips_markup() {
        assert_eq!(
            strip_html("<p>Hello&nbsp;<b>world</b> &amp; friends</p>"),
            "Hello world & friends"
        );
    }

    #[test]
    fn encodes_download_urls() {
        assert_eq!(
            download_url("my-item", "folder/Episode 1 (final).mp4"),
            "https://archive.org/download/my-item/folder/Episode%201%20%28final%29.mp4"
        );
    }
}
