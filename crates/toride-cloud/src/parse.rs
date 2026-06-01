//! Parsing functions for cloud provider output formats.
//!
//! Each cloud provider CLI tool produces structured or semi-structured output
//! (JSON, table, etc.). This module provides parser functions that convert raw
//! output into typed [`FirewallRule`](crate::spec::FirewallRule) and
//! [`SecurityGroup`](crate::spec::SecurityGroup) values.

use crate::error::{Error, Result};
use crate::spec::{PortRange, Protocol, SecurityGroup};

// ---------------------------------------------------------------------------
// AWS parsing
// ---------------------------------------------------------------------------

/// Parse AWS EC2 `describe-security-groups` JSON output into security groups.
///
/// # Errors
///
/// Returns [`Error::ConfigParse`] if the JSON structure is unexpected.
pub fn parse_aws_security_groups(json: &str) -> Result<Vec<SecurityGroup>> {
    let _ = json;
    // TODO: Implement AWS JSON parsing.
    Ok(Vec::new())
}

// ---------------------------------------------------------------------------
// GCP parsing
// ---------------------------------------------------------------------------

/// Parse GCP `gcloud compute firewall-rules list --format=json` output.
///
/// # Errors
///
/// Returns [`Error::ConfigParse`] if the JSON structure is unexpected.
pub fn parse_gcp_firewall_rules(json: &str) -> Result<Vec<SecurityGroup>> {
    let _ = json;
    // TODO: Implement GCP JSON parsing.
    Ok(Vec::new())
}

// ---------------------------------------------------------------------------
// DigitalOcean parsing
// ---------------------------------------------------------------------------

/// Parse DigitalOcean `doctl compute firewall list --format json` output.
///
/// # Errors
///
/// Returns [`Error::ConfigParse`] if the JSON structure is unexpected.
pub fn parse_digitalocean_firewalls(json: &str) -> Result<Vec<SecurityGroup>> {
    let _ = json;
    // TODO: Implement DigitalOcean JSON parsing.
    Ok(Vec::new())
}

// ---------------------------------------------------------------------------
// Hetzner parsing
// ---------------------------------------------------------------------------

/// Parse Hetzner Cloud `hcloud firewall list -o json` output.
///
/// # Errors
///
/// Returns [`Error::ConfigParse`] if the JSON structure is unexpected.
pub fn parse_hetzner_firewalls(json: &str) -> Result<Vec<SecurityGroup>> {
    let _ = json;
    // TODO: Implement Hetzner JSON parsing.
    Ok(Vec::new())
}

// ---------------------------------------------------------------------------
// Generic helpers
// ---------------------------------------------------------------------------

/// Parse a port string like `"80"` or `"8000-9000"` into a [`PortRange`].
///
/// # Errors
///
/// Returns [`Error::ConfigParse`] if the string is not a valid port or range.
pub fn parse_port_range(s: &str) -> Result<PortRange> {
    if let Some((start_str, end_str)) = s.split_once('-') {
        let start = start_str.parse::<u16>().map_err(|_| {
            Error::ConfigParse(format!("invalid port range start: {start_str}"))
        })?;
        let end = end_str.parse::<u16>().map_err(|_| {
            Error::ConfigParse(format!("invalid port range end: {end_str}"))
        })?;
        Ok(PortRange::range(start, end))
    } else {
        let port = s.parse::<u16>().map_err(|_| {
            Error::ConfigParse(format!("invalid port: {s}"))
        })?;
        Ok(PortRange::single(port))
    }
}

/// Parse a protocol string into a [`Protocol`].
///
/// Case-insensitive. Returns [`Protocol::Other`] for unknown protocols.
pub fn parse_protocol(s: &str) -> Protocol {
    match s.to_ascii_lowercase().as_str() {
        "tcp" => Protocol::Tcp,
        "udp" => Protocol::Udp,
        "icmp" => Protocol::Icmp,
        "all" | "-1" => Protocol::All,
        other => {
            if let Ok(n) = other.parse::<u8>() {
                Protocol::Other(n)
            } else {
                Protocol::All
            }
        }
    }
}

/// Detect the provider from raw JSON output and parse accordingly.
///
/// Inspects the structure of the JSON to determine which provider produced it,
/// then delegates to the appropriate parser.
///
/// # Errors
///
/// Returns [`Error::ProviderNotFound`] if the provider cannot be determined.
pub fn parse_auto(json: &str) -> Result<Vec<SecurityGroup>> {
    let _ = json;
    // TODO: Implement auto-detection logic.
    Err(Error::ProviderNotFound("cannot determine cloud provider from output".to_string()))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Protocol;

    // -- parse_port_range ----------------------------------------------------

    #[test]
    fn parse_port_range_single_port() {
        let pr = parse_port_range("80").unwrap();
        assert_eq!(pr.start, 80);
        assert_eq!(pr.end, 80);
        assert!(pr.is_single());
    }

    #[test]
    fn parse_port_range_range() {
        let pr = parse_port_range("8000-9000").unwrap();
        assert_eq!(pr.start, 8000);
        assert_eq!(pr.end, 9000);
        assert!(!pr.is_single());
    }

    #[test]
    fn parse_port_range_invalid_returns_error() {
        assert!(parse_port_range("abc").is_err());
        assert!(parse_port_range("").is_err());
        assert!(parse_port_range("80-abc").is_err());
    }

    #[test]
    fn parse_port_range_boundary_ports() {
        let pr = parse_port_range("1").unwrap();
        assert_eq!(pr.start, 1);

        let pr = parse_port_range("65535").unwrap();
        assert_eq!(pr.start, 65535);
    }

    // -- parse_protocol ------------------------------------------------------

    #[test]
    fn parse_protocol_tcp() {
        assert_eq!(parse_protocol("tcp"), Protocol::Tcp);
    }

    #[test]
    fn parse_protocol_udp() {
        assert_eq!(parse_protocol("udp"), Protocol::Udp);
    }

    #[test]
    fn parse_protocol_icmp() {
        assert_eq!(parse_protocol("icmp"), Protocol::Icmp);
    }

    #[test]
    fn parse_protocol_all() {
        assert_eq!(parse_protocol("all"), Protocol::All);
    }

    #[test]
    fn parse_protocol_case_insensitive() {
        assert_eq!(parse_protocol("TCP"), Protocol::Tcp);
        assert_eq!(parse_protocol("Udp"), Protocol::Udp);
        assert_eq!(parse_protocol("ICMP"), Protocol::Icmp);
        assert_eq!(parse_protocol("ALL"), Protocol::All);
        assert_eq!(parse_protocol("Tcp"), Protocol::Tcp);
    }

    #[test]
    fn parse_protocol_numeric_returns_other() {
        assert_eq!(parse_protocol("47"), Protocol::Other(47));
    }

    #[test]
    fn parse_protocol_minus_one_returns_all() {
        assert_eq!(parse_protocol("-1"), Protocol::All);
    }
}
