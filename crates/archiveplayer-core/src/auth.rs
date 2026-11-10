use crate::error::{CoreError, CoreResult};
use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use rand_core::OsRng;
use regex::Regex;
use std::sync::OnceLock;

pub fn validate_email(email: &str) -> CoreResult<()> {
    static EMAIL: OnceLock<Regex> = OnceLock::new();
    let regex = EMAIL
        .get_or_init(|| Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]{2,}$").expect("valid email regex"));

    if !regex.is_match(email.trim()) {
        return Err(CoreError::Invalid("Enter a valid email address.".into()));
    }
    Ok(())
}

pub fn validate_password(password: &str) -> CoreResult<()> {
    if password.len() < 8 {
        return Err(CoreError::Invalid(
            "Passwords must be at least 8 characters long.".into(),
        ));
    }
    if password.len() > 256 {
        return Err(CoreError::Invalid("That password is too long.".into()));
    }
    Ok(())
}

pub fn hash_password(password: &str) -> CoreResult<String> {
    validate_password(password)?;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| CoreError::Message(format!("Could not secure the password: {error}")))
}

pub fn verify_password(stored_hash: &str, password: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(stored_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_and_verifies() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert!(verify_password(&hash, "correct horse battery staple"));
        assert!(!verify_password(&hash, "wrong password"));
    }

    #[test]
    fn rejects_short_passwords() {
        assert!(hash_password("short").is_err());
    }

    #[test]
    fn validates_emails() {
        assert!(validate_email("person@example.com").is_ok());
        assert!(validate_email("not-an-email").is_err());
    }
}
