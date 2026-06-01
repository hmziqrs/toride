//! `wg-quick` service management via `toride-service`.
//!
//! Provides start/stop/restart operations for WireGuard tunnels using
//! `wg-quick` and integrates with `toride-service` for systemd service
//! management.

use crate::error::Result;

// ---------------------------------------------------------------------------
// WireguardService
// ---------------------------------------------------------------------------

/// Manages `wg-quick` service operations for a WireGuard interface.
///
/// Wraps `wg-quick up/down` commands and optionally delegates to
/// `systemctl` for systemd-managed tunnels.
pub struct WireguardService {
    interface: String,
}

impl WireguardService {
    /// Create a new service manager for the given interface.
    pub fn new(interface: &str) -> Self {
        Self {
            interface: interface.to_owned(),
        }
    }

    /// Returns the interface name.
    pub fn interface(&self) -> &str {
        &self.interface
    }

    /// Returns the systemd service name (`wg-quick@<interface>`).
    pub fn service_name(&self) -> String {
        format!("wg-quick@{}", self.interface)
    }

    /// Bring the interface up using `wg-quick up <interface>`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the command fails.
    pub fn up(&self) -> Result<()> {
        tracing::info!("bringing up WireGuard interface {}", self.interface);
        // TODO: implement via `wg-quick up <interface>`.
        Ok(())
    }

    /// Bring the interface down using `wg-quick down <interface>`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the command fails.
    pub fn down(&self) -> Result<()> {
        tracing::info!("bringing down WireGuard interface {}", self.interface);
        // TODO: implement via `wg-quick down <interface>`.
        Ok(())
    }

    /// Restart the interface (down then up).
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if either command fails.
    pub fn restart(&self) -> Result<()> {
        tracing::info!("restarting WireGuard interface {}", self.interface);
        self.down()?;
        self.up()
    }

    /// Check if the interface is currently up.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the status cannot be determined.
    pub fn is_active(&self) -> Result<bool> {
        tracing::debug!("checking if interface {} is active", self.interface);
        // TODO: implement via `systemctl is-active wg-quick@<interface>`
        // or check if the interface exists.
        Ok(false)
    }

    /// Enable the service to start on boot via systemd.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the command fails.
    pub fn enable(&self) -> Result<()> {
        tracing::info!("enabling WireGuard service {}", self.service_name());
        // TODO: implement via `systemctl enable wg-quick@<interface>`.
        Ok(())
    }

    /// Disable the service from starting on boot.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the command fails.
    pub fn disable(&self) -> Result<()> {
        tracing::info!("disabling WireGuard service {}", self.service_name());
        // TODO: implement via `systemctl disable wg-quick@<interface>`.
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_name_format() {
        let svc = WireguardService::new("wg0");
        assert_eq!(svc.service_name(), "wg-quick@wg0");
        assert_eq!(svc.interface(), "wg0");
    }
}
