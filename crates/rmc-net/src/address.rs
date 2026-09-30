//! Minecraft server address resolution helpers.

use hickory_resolver::Resolver;
use std::net::IpAddr;

pub const DEFAULT_MINECRAFT_PORT: u16 = 25565;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectTarget {
    pub host: String,
    pub port: u16,
    pub srv_resolved: bool,
}

impl ConnectTarget {
    pub fn original(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            srv_resolved: false,
        }
    }
}

pub fn resolve_connect_target(server_address: &str, server_port: u16) -> ConnectTarget {
    if server_port != DEFAULT_MINECRAFT_PORT || server_address.parse::<IpAddr>().is_ok() {
        return ConnectTarget::original(server_address, server_port);
    }

    let Ok(resolver) = Resolver::from_system_conf() else {
        return ConnectTarget::original(server_address, server_port);
    };
    let query = format!("_minecraft._tcp.{server_address}");
    let Ok(lookup) = resolver.srv_lookup(query.as_str()) else {
        return ConnectTarget::original(server_address, server_port);
    };

    let Some(record) = lookup
        .iter()
        .min_by_key(|record| (record.priority(), u16::MAX - record.weight()))
    else {
        return ConnectTarget::original(server_address, server_port);
    };

    let target = record.target().to_utf8();
    let host = target.trim_end_matches('.').to_owned();
    if host.is_empty() {
        return ConnectTarget::original(server_address, server_port);
    }

    ConnectTarget {
        host,
        port: record.port(),
        srv_resolved: true,
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve_connect_target, ConnectTarget, DEFAULT_MINECRAFT_PORT};

    #[test]
    fn skips_srv_for_ip_literals() {
        assert_eq!(
            resolve_connect_target("127.0.0.1", DEFAULT_MINECRAFT_PORT),
            ConnectTarget::original("127.0.0.1", DEFAULT_MINECRAFT_PORT)
        );
    }

    #[test]
    fn skips_srv_for_non_default_ports() {
        assert_eq!(
            resolve_connect_target("hypixel.net", 25566),
            ConnectTarget::original("hypixel.net", 25566)
        );
    }
}
