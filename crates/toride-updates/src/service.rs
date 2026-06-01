//! Service management for unattended-upgrades / dnf-automatic.
//!
//! Provides start/stop/enable/disable operations for the automatic update
//! service, delegating to [`toride_service`] for systemd interaction.

use tracing::info;

use crate::detect::PackageManager;
use crate::error::Result;

// ---------------------------------------------------------------------------
// ServiceManager
// ---------------------------------------------------------------------------

/// Manages the automatic update systemd service.
///
/// On APT systems, manages `unattended-upgrades`. On DNF systems, manages
/// `dnf-automatic.timer`.
pub struct ServiceManager<'a> {
    pkg_mgr: PackageManager,
    _runner: &'a dyn toride_runner::Runner,
}

impl<'a> ServiceManager<'a> {
    /// Create a new service manager.
    pub fn new(runner: &'a dyn toride_runner::Runner) -> Self {
        Self {
            pkg_mgr: crate::detect::detect_package_manager(),
            _runner: runner,
        }
    }

    /// Return the service name for the detected package manager.
    pub fn service_name(&self) -> &'static str {
        match self.pkg_mgr {
            PackageManager::Apt => "unattended-upgrades",
            PackageManager::Dnf => "dnf-automatic.timer",
            PackageManager::Unknown => "unknown",
        }
    }

    /// Return the timer unit name (DNF only).
    pub fn timer_name(&self) -> Option<&'static str> {
        match self.pkg_mgr {
            PackageManager::Dnf => Some("dnf-automatic.timer"),
            _ => None,
        }
    }

    /// Enable and start the automatic update service.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if systemctl fails.
    pub fn enable(&self) -> Result<()> {
        let service = self.service_name();
        info!("Enabling service: {service}");
        // TODO: Delegate to toride_service::ServiceManager.
        Ok(())
    }

    /// Disable and stop the automatic update service.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if systemctl fails.
    pub fn disable(&self) -> Result<()> {
        let service = self.service_name();
        info!("Disabling service: {service}");
        // TODO: Delegate to toride_service::ServiceManager.
        Ok(())
    }

    /// Check whether the service is currently active.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if systemctl fails.
    pub fn is_active(&self) -> Result<bool> {
        // TODO: Delegate to toride_service::ServiceManager.
        Ok(false)
    }

    /// Restart the service.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if systemctl fails.
    pub fn restart(&self) -> Result<()> {
        let service = self.service_name();
        info!("Restarting service: {service}");
        // TODO: Delegate to toride_service::ServiceManager.
        Ok(())
    }
}
