use crate::{API_VERSION, config::Config, session};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::fs;
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
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

    let addresses = lan_addresses();
    let preferred = preferred_address(config, &addresses);
    let mut properties: Vec<(&str, String)> = vec![
        ("name", name.clone()),
        ("id", id),
        ("api", API_VERSION.to_string()),
        ("version", env!("CARGO_PKG_VERSION").to_string()),
        ("path", "/".to_string()),
    ];
    if let Some(address) = preferred {
        // DNS-SD can return every address on every interface. The TV uses this
        // hint to connect to the interface that actually has a route, instead
        // of guessing between Docker bridges or link-local IPv6 addresses.
        properties.push(("addr", address.to_string()));
    }
    if let Some(url) = &config.public_url {
        properties.push(("url", url.clone()));
    }

    let info = ServiceInfo::new(
        SERVICE_TYPE,
        &name,
        &host,
        addresses.as_slice(),
        config.bind.port(),
        &properties[..],
    )?;

    let daemon = ServiceDaemon::new()?;
    let fullname = info.get_fullname().to_string();
    daemon.register(info)?;
    Ok(MdnsGuard { daemon, fullname })
}

/// LAN addresses worth advertising. Loopback and link-local IPv6 are omitted:
/// they are unusable or ambiguous on a television, and `enable_addr_auto`
/// would otherwise publish every Docker bridge on the host.
fn lan_addresses() -> Vec<IpAddr> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|interface| !interface.is_loopback())
        .map(|interface| interface.ip())
        .filter(|address| match address {
            IpAddr::V4(address) => address.is_private() && !address.is_link_local(),
            IpAddr::V6(_) => false,
        })
        .collect()
}

/// Prefer the address the OS would use for an outbound packet. A UDP connect
/// does not send anything, but it asks the routing table which source address
/// is appropriate. A server explicitly bound to one interface wins.
fn preferred_address(config: &Config, addresses: &[IpAddr]) -> Option<IpAddr> {
    let bind = config.bind.ip();
    if !bind.is_unspecified() && !bind.is_loopback() {
        return Some(bind);
    }
    primary_ipv4()
        .map(IpAddr::V4)
        .filter(|address| addresses.contains(address))
        .or_else(|| addresses.first().copied())
}

fn primary_ipv4() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(address) if !address.is_loopback() => Some(address),
        _ => None,
    }
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
    fn an_explicit_bind_address_wins_for_the_txt_hint() {
        let mut config = config();
        config.bind = "192.168.1.7:8080".parse().unwrap();
        let addresses = [IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2))];
        assert_eq!(
            preferred_address(&config, &addresses),
            Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 7)))
        );
    }

    #[test]
    fn advertised_addresses_are_lan_ipv4_only() {
        for address in lan_addresses() {
            match address {
                IpAddr::V4(address) => {
                    assert!(address.is_private() && !address.is_link_local());
                }
                IpAddr::V6(_) => panic!("IPv6 should not be advertised for LAN discovery"),
            }
        }
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
