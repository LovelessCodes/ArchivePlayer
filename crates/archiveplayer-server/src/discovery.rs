use crate::{API_VERSION, config::Config, session};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::fs;
use std::path::Path;

const SERVICE_TYPE: &str = "_archiveplayer._tcp.local.";
const SERVER_ID_FILE: &str = "server-id";

pub struct MdnsGuard {
    daemon: ServiceDaemon,
    fullname: String,
}

impl Drop for MdnsGuard {
    fn drop(&mut self) {
        let _ = self.daemon.unregister(&self.fullname);
        let _ = self.daemon.shutdown();
    }
}

/// Advertise the server on the local network. Discovery is best-effort: a
/// failure must never stop the HTTP server from starting.
pub fn start(config: &Config, data_dir: &Path) -> Option<MdnsGuard> {
    if !config.mdns {
        return None;
    }
    if config.bind.ip().is_loopback() {
        tracing::debug!(
            "mDNS discovery is disabled for loopback bind {}; use 0.0.0.0 for LAN discovery",
            config.bind
        );
        return None;
    }

    match register(config, data_dir) {
        Ok(guard) => {
            tracing::info!(
                "advertising \"{}\" on _archiveplayer._tcp.local port {}",
                server_name(config),
                config.bind.port()
            );
            Some(guard)
        }
        Err(error) => {
            tracing::warn!("mDNS discovery could not start: {error}");
            None
        }
    }
}

fn register(config: &Config, data_dir: &Path) -> Result<MdnsGuard, Box<dyn std::error::Error>> {
    let id = stable_server_id(data_dir)?;
    let name = server_name(config);
    let short_id: String = id.chars().take(12).collect();
    let host = format!("archiveplayer-{short_id}.local.");

    let mut properties: Vec<(&str, String)> = vec![
        ("name", name.clone()),
        ("id", id),
        ("api", API_VERSION.to_string()),
        ("version", env!("CARGO_PKG_VERSION").to_string()),
        ("path", "/".to_string()),
    ];
    if let Some(url) = &config.public_url {
        properties.push(("url", url.clone()));
    }

    let info = ServiceInfo::new(
        SERVICE_TYPE,
        &name,
        &host,
        "",
        config.bind.port(),
        &properties[..],
    )?
    .enable_addr_auto();

    let daemon = ServiceDaemon::new()?;
    let fullname = info.get_fullname().to_string();
    daemon.register(info)?;
    Ok(MdnsGuard { daemon, fullname })
}

fn server_name(config: &Config) -> String {
    let candidate = config
        .server_name
        .clone()
        .or_else(|| std::env::var("HOSTNAME").ok())
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .unwrap_or_else(|| "ArchivePlayer".to_string());
    let name: String = candidate
        .chars()
        .filter(|character| !character.is_control())
        .take(64)
        .collect();
    let name = name.trim();
    if name.is_empty() {
        "ArchivePlayer".to_string()
    } else {
        name.to_string()
    }
}

/// The ID is stable across restarts and lets a TV remember the same server
/// even when its address changes. It is intentionally only used for discovery.
fn stable_server_id(data_dir: &Path) -> std::io::Result<String> {
    let path = data_dir.join(SERVER_ID_FILE);
    if let Ok(value) = fs::read_to_string(&path) {
        let value = value.trim();
        if (12..=128).contains(&value.len()) {
            return Ok(value.to_string());
        }
    }

    let value = session::new_token();
    fs::create_dir_all(data_dir)?;
    fs::write(&path, &value)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn config() -> Config {
        Config {
            bind: "0.0.0.0:8080".parse().unwrap(),
            data_dir: PathBuf::from("data"),
            static_dir: PathBuf::from("dist"),
            secure_cookies: false,
            allow_registration: false,
            session_days: 30,
            device_session_days: 180,
            public_url: Some("https://media.example.com".into()),
            cors_origins: vec![],
            mdns: true,
            server_name: Some("Living Room NAS".into()),
            trust_proxy: false,
            rate_limit_per_minute: 0,
            auth_rate_limit_per_minute: 0,
            device_rate_limit_per_minute: 0,
        }
    }

    #[test]
    fn server_ids_are_stable() {
        let dir = tempfile::tempdir().unwrap();
        let first = stable_server_id(dir.path()).unwrap();
        let second = stable_server_id(dir.path()).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
    }

    #[test]
    fn service_name_is_bounded_and_falls_back() {
        let mut config = config();
        config.server_name = Some("  ".into());
        assert_eq!(server_name(&config), "ArchivePlayer");
        config.server_name = Some("x".repeat(100));
        assert_eq!(server_name(&config).chars().count(), 64);
    }
}
