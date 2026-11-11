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
    pub device_session_days: i64,
    pub public_url: Option<String>,
    /// Exact browser origins allowed to call the API from another origin.
    /// Native Android apps do not need this; webOS and other web runtimes do.
    pub cors_origins: Vec<String>,
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

        let session_days = env_days("ARCHIVEPLAYER_SESSION_DAYS", 30, 365)?;
        let device_session_days = env_days("ARCHIVEPLAYER_DEVICE_SESSION_DAYS", 180, 3650)?;

        let public_url = std::env::var("ARCHIVEPLAYER_PUBLIC_URL")
            .ok()
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty());

        let cors_origins = std::env::var("ARCHIVEPLAYER_CORS_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(|origin| {
                if origin == "null"
                    || origin.starts_with("http://")
                    || origin.starts_with("https://")
                {
                    Ok(origin.to_string())
                } else {
                    Err(format!(
                        "Invalid ARCHIVEPLAYER_CORS_ORIGINS entry: {origin}"
                    ))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            bind,
            data_dir,
            static_dir,
            secure_cookies,
            allow_registration,
            session_days,
            device_session_days,
            public_url,
            cors_origins,
        })
    }
}

fn env_days(name: &str, default: i64, max: i64) -> Result<i64, String> {
    Ok(std::env::var(name)
        .ok()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("Invalid {name}: {error}"))
        })
        .transpose()?
        .unwrap_or(default)
        .clamp(1, max))
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
