use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: SocketAddr,
    pub data_dir: PathBuf,
    pub static_dir: PathBuf,
    pub secure_cookies: bool,
    pub allow_registration: bool,
    pub session_days: i64,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let bind = std::env::var("ARCHIVEPLAYER_BIND")
            .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
            .parse()
            .map_err(|error| format!("Invalid ARCHIVEPLAYER_BIND: {error}"))?;

        let data_dir = std::env::var_os("ARCHIVEPLAYER_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("data"));

        let static_dir = std::env::var_os("ARCHIVEPLAYER_STATIC_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("dist"));

        let secure_cookies = env_bool("ARCHIVEPLAYER_SECURE_COOKIES", false)?;
        let allow_registration = env_bool("ARCHIVEPLAYER_ALLOW_REGISTRATION", false)?;

        let session_days = std::env::var("ARCHIVEPLAYER_SESSION_DAYS")
            .ok()
            .map(|value| {
                value
                    .parse::<i64>()
                    .map_err(|error| format!("Invalid ARCHIVEPLAYER_SESSION_DAYS: {error}"))
            })
            .transpose()?
            .unwrap_or(30)
            .clamp(1, 365);

        Ok(Self {
            bind,
            data_dir,
            static_dir,
            secure_cookies,
            allow_registration,
            session_days,
        })
    }
}

fn env_bool(name: &str, default: bool) -> Result<bool, String> {
    match std::env::var(name) {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            other => Err(format!(
                "Invalid {name}: expected true or false, got {other}"
            )),
        },
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(format!("Invalid {name}: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_boolean_values() {
        assert!(env_bool("ARCHIVEPLAYER_TEST_MISSING", true).unwrap());
        assert!(!env_bool("ARCHIVEPLAYER_TEST_MISSING", false).unwrap());
    }
}
