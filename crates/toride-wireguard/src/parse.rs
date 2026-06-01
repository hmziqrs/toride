//! Parsers for WireGuard output and configuration files.
//!
//! Provides parsers for:
//! - `wg show` output (tab-separated key-value pairs)
//! - `wg showconf` output (INI-like format)
//! - Interface `.conf` files (INI format)

use crate::error::{Error, Result};
use crate::spec::{PeerSpec, WireguardSpec};

// ---------------------------------------------------------------------------
// parse_wg_show
// ---------------------------------------------------------------------------

/// Parse the output of `wg show` into a list of interface summaries.
///
/// `wg show` prints tab-separated fields. This parser extracts interface names
/// and their associated metadata into a simplified representation.
///
/// # Errors
///
/// Returns [`Error::ConfigParse`] if the output cannot be parsed.
pub fn parse_wg_show(output: &str) -> Result<Vec<WgShowEntry>> {
    let mut entries = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            entries.push(WgShowEntry {
                interface: parts[0].to_owned(),
                public_key: parts.get(1).unwrap_or(&"").to_string(),
                listen_port: parts
                    .get(2)
                    .and_then(|s| s.parse::<u16>().ok())
                    .unwrap_or(0),
            });
        }
    }
    Ok(entries)
}

/// A simplified entry parsed from `wg show` output.
#[derive(Debug, Clone)]
pub struct WgShowEntry {
    /// Interface name (e.g. `wg0`).
    pub interface: String,
    /// Public key of the interface.
    pub public_key: String,
    /// Listen port.
    pub listen_port: u16,
}

// ---------------------------------------------------------------------------
// parse_wg_showconf
// ---------------------------------------------------------------------------

/// Parse the output of `wg showconf <interface>` into a [`WireguardSpec`].
///
/// The output is in INI-like format with `[Interface]` and `[Peer]` sections.
///
/// # Errors
///
/// Returns [`Error::ConfigParse`] if the output cannot be parsed.
pub fn parse_wg_showconf(interface: &str, output: &str) -> Result<WireguardSpec> {
    parse_interface_conf(interface, output)
}

// ---------------------------------------------------------------------------
// parse_interface_conf
// ---------------------------------------------------------------------------

/// Parse an INI-format WireGuard interface config file.
///
/// Supports the standard `[Interface]` and `[Peer]` sections with their
/// respective keys: `Address`, `ListenPort`, `PrivateKey`, `DNS`,
/// `PublicKey`, `AllowedIPs`, `Endpoint`, `PersistentKeepalive`.
///
/// # Errors
///
/// Returns [`Error::ConfigParse`] if the config contains invalid syntax.
pub fn parse_interface_conf(interface_name: &str, content: &str) -> Result<WireguardSpec> {
    let mut spec = WireguardSpec::new(interface_name.to_owned(), String::new());
    let mut current_section: Option<String> = None;
    let mut current_peer: Option<PeerSpec> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip blank lines and comments.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Section headers.
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Flush any in-progress peer.
            if let Some(peer) = current_peer.take() {
                spec.peers.push(peer);
            }
            current_section = Some(trimmed[1..trimmed.len() - 1].trim().to_owned());
            if current_section.as_deref() == Some("Peer") {
                current_peer = Some(PeerSpec::new(String::new(), Vec::new()));
            }
            continue;
        }

        // Key = Value pairs.
        let (key, value) = match trimmed.split_once('=') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };

        match current_section.as_deref() {
            Some("Interface") => match key {
                "Address" => spec.address = value.to_owned(),
                "ListenPort" => {
                    spec.listen_port = value.parse().map_err(|_| {
                        Error::ConfigParse(format!("invalid ListenPort: {value}"))
                    })?;
                }
                "PrivateKey" => spec.private_key = Some(value.to_owned()),
                "DNS" => spec.dns = Some(value.to_owned()),
                _ => {} // ignore unknown keys
            },
            Some("Peer") => {
                if let Some(ref mut peer) = current_peer {
                    match key {
                        "PublicKey" => peer.public_key = value.to_owned(),
                        "AllowedIPs" => {
                            peer.allowed_ips =
                                value.split(',').map(|s| s.trim().to_owned()).collect();
                        }
                        "Endpoint" => peer.endpoint = Some(value.to_owned()),
                        "PersistentKeepalive" => {
                            peer.persistent_keepalive =
                                Some(value.parse().map_err(|_| {
                                    Error::ConfigParse(format!(
                                        "invalid PersistentKeepalive: {value}"
                                    ))
                                })?);
                        }
                        _ => {} // ignore unknown keys
                    }
                }
            }
            _ => {
                return Err(Error::ConfigParse(format!(
                    "key-value pair outside of section: {trimmed}"
                )));
            }
        }
    }

    // Flush the last peer if present.
    if let Some(peer) = current_peer.take() {
        spec.peers.push(peer);
    }

    Ok(spec)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_interface_conf() {
        let conf = "\
[Interface]
Address = 10.0.0.1/24
ListenPort = 51820
PrivateKey = abc123base64==
DNS = 1.1.1.1

[Peer]
PublicKey = peerpubkey==
AllowedIPs = 10.0.0.2/32
Endpoint = 203.0.113.1:51820
PersistentKeepalive = 25
";
        let spec = parse_interface_conf("wg0", conf).unwrap();
        assert_eq!(spec.name, "wg0");
        assert_eq!(spec.address, "10.0.0.1/24");
        assert_eq!(spec.listen_port, 51820);
        assert_eq!(spec.private_key.as_deref(), Some("abc123base64=="));
        assert_eq!(spec.dns.as_deref(), Some("1.1.1.1"));
        assert_eq!(spec.peers.len(), 1);
        let peer = &spec.peers[0];
        assert_eq!(peer.public_key, "peerpubkey==");
        assert_eq!(peer.allowed_ips, vec!["10.0.0.2/32"]);
        assert_eq!(peer.endpoint.as_deref(), Some("203.0.113.1:51820"));
        assert_eq!(peer.persistent_keepalive, Some(25));
    }

    #[test]
    fn parse_multiple_peers() {
        let conf = "\
[Interface]
Address = 10.0.0.1/24

[Peer]
PublicKey = key1==
AllowedIPs = 10.0.0.2/32

[Peer]
PublicKey = key2==
AllowedIPs = 10.0.0.3/32, fd00::3/128
";
        let spec = parse_interface_conf("wg0", conf).unwrap();
        assert_eq!(spec.peers.len(), 2);
        assert_eq!(spec.peers[1].allowed_ips.len(), 2);
    }

    #[test]
    fn parse_empty_conf() {
        let spec = parse_interface_conf("wg0", "").unwrap();
        assert_eq!(spec.name, "wg0");
        assert!(spec.address.is_empty());
        assert!(spec.peers.is_empty());
    }

    #[test]
    fn parse_wg_show_output() {
        let output = "wg0\tABC123==\t51820\nwg1\tDEF456==\t51821\n";
        let entries = parse_wg_show(output).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].interface, "wg0");
        assert_eq!(entries[0].listen_port, 51820);
        assert_eq!(entries[1].interface, "wg1");
    }
}
