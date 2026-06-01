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
