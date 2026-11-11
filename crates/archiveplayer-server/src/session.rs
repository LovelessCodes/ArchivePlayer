use crate::config::Config;
use axum::http::{HeaderMap, header};
use axum_extra::extract::cookie::{Cookie, SameSite};
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use time::Duration;

pub const COOKIE_NAME: &str = "archiveplayer_session";

const USER_CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// Eight characters from a 32-symbol alphabet, formatted as `ABCD-EFGH`.
/// 40 bits of entropy is enough for a ten-minute, rate-limited pairing window.
pub fn new_user_code() -> String {
    let mut bytes = [0_u8; 8];
    OsRng.fill_bytes(&mut bytes);
    let code: String = bytes
        .iter()
        .map(|byte| USER_CODE_ALPHABET[*byte as usize % USER_CODE_ALPHABET.len()] as char)
        .collect();
    format!("{}-{}", &code[..4], &code[4..])
}

/// Accepts `abcd-efgh`, `ABCD EFGH`, or a QR URL-encoded variant and returns
/// the canonical eight-character code.
pub fn normalize_user_code(value: &str) -> Option<String> {
    let code: String = value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .map(|character| character.to_ascii_uppercase())
        .collect();
    if code.len() != 8 || !code.bytes().all(|byte| USER_CODE_ALPHABET.contains(&byte)) {
        return None;
    }
    Some(code)
}

pub fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = token.trim();
    (!token.is_empty()).then(|| token.to_string())
}

pub fn request_token(headers: &HeaderMap) -> Option<String> {
    if let Some(token) = bearer_token(headers) {
        return Some(token);
    }
    let jar = axum_extra::extract::cookie::CookieJar::from_headers(headers);
    jar.get(COOKIE_NAME)
        .map(|cookie| cookie.value().to_string())
}

pub fn browser_label(headers: &HeaderMap) -> String {
    let agent = headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let browser = if agent.contains("Edg/") {
        "Edge"
    } else if agent.contains("OPR/") || agent.contains("Opera") {
        "Opera"
    } else if agent.contains("Firefox/") {
        "Firefox"
    } else if agent.contains("Chrome/") {
        "Chrome"
    } else if agent.contains("Safari/") {
        "Safari"
    } else {
        "Browser"
    };
    let os = if agent.contains("iPhone") {
        "iPhone"
    } else if agent.contains("iPad") {
        "iPad"
    } else if agent.contains("Android") {
        "Android"
    } else if agent.contains("Windows") {
        "Windows"
    } else if agent.contains("Mac OS X") || agent.contains("Macintosh") {
        "macOS"
    } else if agent.contains("Linux") {
        "Linux"
    } else {
        "web"
    };
    format!("{browser} on {os}")
}

pub fn new_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let mut token = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut token, "{byte:02x}").expect("writing to a String cannot fail");
    }
    token
}

pub fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    let mut hash = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut hash, "{byte:02x}").expect("writing to a String cannot fail");
    }
    hash
}

pub fn build_cookie(token: String, remember: bool, config: &Config) -> Cookie<'static> {
    let mut cookie = Cookie::new(COOKIE_NAME, token);
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_secure(config.secure_cookies);
    if remember {
        cookie.set_max_age(Some(Duration::days(config.session_days)));
    }
    cookie
}

pub fn removal_cookie() -> Cookie<'static> {
    let mut cookie = Cookie::new(COOKIE_NAME, "");
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_max_age(Some(Duration::seconds(0)));
    cookie
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_random_and_hashable() {
        let a = new_token();
        let b = new_token();
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert_eq!(hash_token(&a).len(), 64);
        assert_ne!(hash_token(&a), hash_token(&b));
    }

    #[test]
    fn user_codes_normalize_and_validate() {
        let code = new_user_code();
        assert_eq!(code.len(), 9);
        assert_eq!(
            code.chars().filter(|character| *character == '-').count(),
            1
        );
        assert_eq!(
            normalize_user_code(&code.replace('-', " ")),
            Some(code.replace('-', ""))
        );
        assert_eq!(normalize_user_code("too-short"), None);
        assert_eq!(normalize_user_code("IIII-OOOO"), None);
    }
}
