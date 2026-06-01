//! Service management layer for backup scheduling.
//!
//! Manages systemd services and timers related to backup operations.
//! Delegates to the [`toride_service`] crate for systemd interactions when
//! available.

use crate::{Error, Result};

// ---------------------------------------------------------------------------
// BackupServiceManager
// ---------------------------------------------------------------------------

/// Manages the backup-related systemd services and timers.
///
/// Provides a typed interface for starting, stopping, enabling, and
/// querying the status of backup timer units.
pub struct BackupServiceManager {
    /// The systemd unit name prefix for backup timers.
    prefix: String,
}

impl BackupServiceManager {
    /// Create a manager with the default prefix `"toride-backup-"`.
    pub fn new() -> Self {
        Self {
            prefix: "toride-backup-".to_owned(),
        }
    }

    /// Create a manager with a custom unit name prefix.
    pub fn with_prefix(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_owned(),
        }
    }

    /// Build the timer unit name for a backup job.
    pub fn timer_unit(&self, name: &str) -> String {
        format!("{}{}.timer", self.prefix, name)
    }

    /// Build the service unit name for a backup job.
    pub fn service_unit(&self, name: &str) -> String {
        format!("{}{}.service", self.prefix, name)
    }

    // -----------------------------------------------------------------------
    // Lifecycle operations
    // -----------------------------------------------------------------------

    /// Start the backup timer for the given job.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if systemctl fails.
    pub fn start_timer(&self, name: &str) -> Result<()> {
        let unit = self.timer_unit(name);
        tracing::info!(unit = %unit, "starting backup timer");
        // TODO: delegate to toride-service or run systemctl.
        Err(Error::CommandFailed("not yet implemented".into()))
    }

    /// Stop the backup timer for the given job.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if systemctl fails.
    pub fn stop_timer(&self, name: &str) -> Result<()> {
        let unit = self.timer_unit(name);
        tracing::info!(unit = %unit, "stopping backup timer");
        // TODO: delegate to toride-service or run systemctl.
        Err(Error::CommandFailed("not yet implemented".into()))
    }

    /// Enable the backup timer to start at boot.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if systemctl fails.
    pub fn enable_timer(&self, name: &str) -> Result<()> {
        let unit = self.timer_unit(name);
        tracing::info!(unit = %unit, "enabling backup timer");
        // TODO: delegate to toride-service or run systemctl.
        Err(Error::CommandFailed("not yet implemented".into()))
    }

    /// Check whether the backup timer is currently active.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if systemctl fails.
    pub fn is_timer_active(&self, name: &str) -> Result<bool> {
        let unit = self.timer_unit(name);
        tracing::debug!(unit = %unit, "checking timer status");
        // TODO: delegate to toride-service or run systemctl.
        let _ = &unit;
        Ok(false)
    }
}

impl Default for BackupServiceManager {
    fn default() -> Self {
        Self::new()
    }
}
