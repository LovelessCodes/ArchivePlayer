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
    /// Advertise the server on the local network with mDNS/DNS-SD.
    pub mdns: bool,
    /// Friendly name shown by LAN discovery. Defaults to the machine hostname.
    pub server_name: Option<String>,
    /// Trust `X-Forwarded-For` / `X-Real-IP` when rate limiting and building
    /// pairing links. Only enable this when a trusted reverse proxy overwrites
    /// those headers.
    pub trust_proxy: bool,
    /// Requests per minute allowed for general API traffic. 0 disables it.
    pub rate_limit_per_minute: u32,
    /// Requests per minute allowed for login and registration. 0 disables it.
    pub auth_rate_limit_per_minute: u32,
    /// Requests per minute allowed for device pairing endpoints. 0 disables it.
    pub device_rate_limit_per_minute: u32,
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

        let mdns = env_bool("ARCHIVEPLAYER_MDNS", true)?;
        let server_name = std::env::var("ARCHIVEPLAYER_SERVER_NAME")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let trust_proxy = env_bool("ARCHIVEPLAYER_TRUST_PROXY", false)?;
        let rate_limit_per_minute = env_u32("ARCHIVEPLAYER_RATE_LIMIT_PER_MINUTE", 240)?;
        let auth_rate_limit_per_minute = env_u32("ARCHIVEPLAYER_AUTH_RATE_LIMIT_PER_MINUTE", 10)?;
        let device_rate_limit_per_minute =
            env_u32("ARCHIVEPLAYER_DEVICE_RATE_LIMIT_PER_MINUTE", 60)?;

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
            mdns,
            server_name,
            trust_proxy,
            rate_limit_per_minute,
            auth_rate_limit_per_minute,
            device_rate_limit_per_minute,
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

fn env_u32(name: &str, default: u32) -> Result<u32, String> {
    Ok(std::env::var(name)
        .ok()
        .map(|value| {
            value
                .trim()
                .parse::<u32>()
                .map(|value| value.min(100_000))
                .map_err(|error| format!("Invalid {name}: {error}"))
        })
        .transpose()?
        .unwrap_or(default))
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
