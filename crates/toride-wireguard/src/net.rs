//! Network interface helpers for WireGuard.
//!
//! Provides utilities for querying and managing WireGuard network interfaces,
//! including listing interfaces, checking interface status, and resolving
//! interface addresses.

use crate::error::Result;
use crate::validate::validate_interface_name;

// ---------------------------------------------------------------------------
// InterfaceStatus
// ---------------------------------------------------------------------------

/// Status of a WireGuard network interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterfaceStatus {
    /// Interface exists and is up.
    Up,
    /// Interface exists but is down.
    Down,
    /// Interface does not exist.
    NotFound,
}

// ---------------------------------------------------------------------------
// NetworkInterface helpers
// ---------------------------------------------------------------------------

/// Check whether a WireGuard interface exists on the system.
///
/// Uses `ip link show <name>` to determine presence.
///
/// # Errors
///
/// Returns [`Error::CommandFailed`] if the check cannot be performed.
pub fn interface_exists(name: &str) -> Result<bool> {
    validate_interface_name(name)?;
    // TODO: implement via `ip link show` or netlink.
    tracing::debug!("checking existence of interface: {name}");
    Ok(false)
}

/// Get the current status of a WireGuard interface.
///
/// # Errors
///
/// Returns [`Error::CommandFailed`] if the status cannot be determined.
pub fn interface_status(name: &str) -> Result<InterfaceStatus> {
    validate_interface_name(name)?;
    // TODO: implement via `ip link show <name>` and check operstate.
    tracing::debug!("querying status of interface: {name}");
    Ok(InterfaceStatus::NotFound)
}

/// List all WireGuard interfaces currently on the system.
///
/// Discovers interfaces by listing `/sys/class/net/` entries whose `type` file
/// contains the WireGuard device type indicator.
///
/// # Errors
///
/// Returns [`Error::Io`] if the sysfs directory cannot be read.
pub fn list_wireguard_interfaces() -> Result<Vec<String>> {
    // TODO: implement via /sys/class/net/*/device/type or `wg show interfaces`.
    tracing::debug!("listing WireGuard interfaces");
    Ok(Vec::new())
}

/// Get the transfer statistics (bytes sent/received) for an interface.
///
/// # Errors
///
/// Returns [`Error::InterfaceNotFound`] if the interface does not exist.
pub fn interface_stats(name: &str) -> Result<InterfaceStats> {
    validate_interface_name(name)?;
    // TODO: implement via `wg show <name> transfer`.
    tracing::debug!("querying transfer stats for interface: {name}");
    Ok(InterfaceStats {
        bytes_received: 0,
        bytes_sent: 0,
    })
}

/// Transfer statistics for a WireGuard interface.
#[derive(Debug, Clone, Default)]
pub struct InterfaceStats {
    /// Total bytes received.
    pub bytes_received: u64,
    /// Total bytes sent.
    pub bytes_sent: u64,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_name_before_checking() {
        // Should fail validation, not reach the implementation.
        let result = interface_exists("eth0");
        assert!(result.is_err());
    }

    #[test]
    fn stats_default() {
        let stats = InterfaceStats::default();
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.bytes_sent, 0);
    }
}
