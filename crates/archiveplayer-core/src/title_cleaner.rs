use regex::Regex;
use std::sync::OnceLock;

fn re(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("valid regex"))
}

/// Transforms messy Archive.org identifiers and titles into clean,
/// TMDB-friendly search queries.
pub fn clean_for_search(title: &str) -> String {
    if title.trim().is_empty() {
        return String::new();
    }

    let uploader = re(&UPLOADER_SUFFIX, r"(?i)[-_]?dktale.*");
    let trailing_numeric = re(&TRAILING_NUMERIC, r"(?i)_\d+$");
    let parenthesized = re(&PARENTHESIZED, r"\s*\([^)]*\)");
    let standard_marker = re(&STANDARD_MARKER, r"(?i)\s*-\s*S\d+E\d+.*");
    let alternate_marker = re(&ALTERNATE_MARKER, r"(?i)\s*\d+x\d+.*");
    let whitespace = re(&WHITESPACE, r"\s+");

    let mut cleaned = uploader.replace_all(title, "").trim().to_string();
    cleaned = trailing_numeric
        .replace_all(&cleaned, "")
        .trim()
        .to_string();
    cleaned = parenthesized.replace_all(&cleaned, "").trim().to_string();
    cleaned = standard_marker.replace_all(&cleaned, "").trim().to_string();
    cleaned = alternate_marker
        .replace_all(&cleaned, "")
        .trim()
        .to_string();
    cleaned = cleaned.replace(['-', '_'], " ");
    cleaned = whitespace.replace_all(&cleaned, " ").trim().to_string();
    cleaned
}

static UPLOADER_SUFFIX: OnceLock<Regex> = OnceLock::new();
static TRAILING_NUMERIC: OnceLock<Regex> = OnceLock::new();
static PARENTHESIZED: OnceLock<Regex> = OnceLock::new();
static STANDARD_MARKER: OnceLock<Regex> = OnceLock::new();
static ALTERNATE_MARKER: OnceLock<Regex> = OnceLock::new();
static WHITESPACE: OnceLock<Regex> = OnceLock::new();

#[cfg(test)]
mod tests {
    use super::clean_for_search;

    #[test]
    fn strips_uploader_suffix() {
        assert_eq!(
            clean_for_search("bakugan-battle-brawlers-dktale_202604"),
            "bakugan battle brawlers"
        );
        assert_eq!(
            clean_for_search("ultimate-spider-man-dktale"),
            "ultimate spider man"
        );
    }

    #[test]
    fn strips_year_and_episode_markers() {
        assert_eq!(clean_for_search("Chowder (2007)"), "Chowder");
        assert_eq!(clean_for_search("Show-Name-S01E05.mkv"), "Show Name");
        assert_eq!(clean_for_search("Show Name 1x05"), "Show Name");
    }

    #[test]
    fn normalizes_separators() {
        assert_eq!(clean_for_search("The__Twilight__Zone"), "The Twilight Zone");
    }
}
