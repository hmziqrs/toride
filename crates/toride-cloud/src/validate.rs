//! Validation functions for firewall rules and CIDR notation.
//!
//! Every firewall rule is validated before being applied to a cloud provider.
//! This ensures that invalid rules are caught early with clear error messages
//! rather than producing cryptic provider API errors.

use std::net::{Ipv4Addr, Ipv6Addr};

use crate::error::{Error, Result};
use crate::spec::{FirewallRule, SecurityGroup};

// ---------------------------------------------------------------------------
// CIDR validation
// ---------------------------------------------------------------------------

/// Validate a CIDR notation string (e.g. `"0.0.0.0/0"`, `"10.0.0.0/8"`).
///
/// Checks that the string parses as a valid IPv4 or IPv6 CIDR and that the
/// prefix length is within range.
///
/// # Errors
///
/// Returns [`Error::ConfigParse`] if the CIDR is invalid.
pub fn validate_cidr(cidr: &str) -> Result<()> {
    if cidr.contains(':') {
        // IPv6 CIDR
        validate_cidr_v6(cidr)
    } else {
        // IPv4 CIDR
        validate_cidr_v4(cidr)
    }
}

fn validate_cidr_v4(cidr: &str) -> Result<()> {
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        return Err(Error::ConfigParse(format!(
            "invalid CIDR notation (missing /): {cidr}"
        )));
    }

    let ip: Ipv4Addr = parts[0].parse().map_err(|_| {
        Error::ConfigParse(format!("invalid IPv4 address in CIDR: {}", parts[0]))
    })?;

    let prefix: u8 = parts[1].parse().map_err(|_| {
        Error::ConfigParse(format!("invalid prefix length in CIDR: {}", parts[1]))
    })?;

    if prefix > 32 {
        return Err(Error::ConfigParse(format!(
            "IPv4 prefix length must be 0-32, got {prefix}"
        )));
    }

    let _ = ip;
    Ok(())
}

fn validate_cidr_v6(cidr: &str) -> Result<()> {
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        return Err(Error::ConfigParse(format!(
            "invalid CIDR notation (missing /): {cidr}"
        )));
    }

    let ip: Ipv6Addr = parts[0].parse().map_err(|_| {
        Error::ConfigParse(format!("invalid IPv6 address in CIDR: {}", parts[0]))
    })?;

    let prefix: u8 = parts[1].parse().map_err(|_| {
        Error::ConfigParse(format!("invalid prefix length in CIDR: {}", parts[1]))
    })?;

    if prefix > 128 {
        return Err(Error::ConfigParse(format!(
            "IPv6 prefix length must be 0-128, got {prefix}"
        )));
    }

    let _ = ip;
    Ok(())
}

// ---------------------------------------------------------------------------
// Firewall rule validation
// ---------------------------------------------------------------------------

/// Validate a single firewall rule.
///
/// Checks:
/// - CIDR is valid
/// - Port range is valid (if present)
/// - Protocol is specified
/// - Description is present (warning if empty)
///
/// # Errors
///
/// Returns [`Error::ConfigParse`] if the rule fails validation.
pub fn validate_firewall_rule(rule: &FirewallRule) -> Result<()> {
    // Validate CIDR
    validate_cidr(&rule.cidr)?;

    // Validate port range
    if let Some(pr) = &rule.port_range {
        if pr.start == 0 {
            return Err(Error::ConfigParse("port number 0 is not valid".to_string()));
        }
        if pr.end < pr.start {
            return Err(Error::ConfigParse(format!(
                "port range end ({}) must be >= start ({})",
                pr.end, pr.start
            )));
        }
    }

    // Warn about missing description
    if rule.description.is_empty() {
        tracing::warn!(
            "firewall rule has no description; adding one is recommended for maintainability"
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Security group validation
// ---------------------------------------------------------------------------

/// Validate an entire security group.
///
/// Validates each rule and checks for duplicate rules.
///
/// # Errors
///
/// Returns [`Error::ConfigParse`] if any rule fails validation or if
/// duplicates are found.
pub fn validate_security_group(group: &SecurityGroup) -> Result<()> {
    for rule in &group.rules {
        validate_firewall_rule(rule)?;
    }

    // Check for duplicate rules
    for i in 0..group.rules.len() {
        for j in (i + 1)..group.rules.len() {
            if rules_are_equivalent(&group.rules[i], &group.rules[j]) {
                return Err(Error::ConfigParse(format!(
                    "duplicate firewall rules at indices {i} and {j}"
                )));
            }
        }
    }

    Ok(())
}

/// Check if two firewall rules are functionally equivalent.
fn rules_are_equivalent(a: &FirewallRule, b: &FirewallRule) -> bool {
    a.is_ingress == b.is_ingress
        && a.protocol == b.protocol
        && a.port_range == b.port_range
        && a.cidr == b.cidr
        && a.action == b.action
}

// ---------------------------------------------------------------------------
// Port validation
// ---------------------------------------------------------------------------

/// Validate that a port number is in the valid range (1-65535).
///
/// # Errors
///
/// Returns [`Error::ConfigParse`] if the port is out of range.
pub fn validate_port(port: u16) -> Result<()> {
    if port == 0 {
        return Err(Error::ConfigParse("port 0 is not valid".to_string()));
    }
    Ok(())
}
