//! Tailscale service lifecycle management.
//!
//! Provides [`TailscaleService`] for managing the `tailscaled` system service:
//! starting, stopping, restarting, and querying the service status.

use crate::Result;

// ---------------------------------------------------------------------------
// TailscaleService
// ---------------------------------------------------------------------------

/// Manager for the `tailscaled` system service.
///
/// `TailscaleService` wraps system service operations (via `systemctl` or
/// equivalent) to manage the Tailscale daemon lifecycle.
///
/// # Example
///
/// ```ignore
/// use toride_tailscale::service::TailscaleService;
///
/// let svc = TailscaleService::new();
/// svc.restart()?;
/// let active = svc.is_active()?;
/// ```
pub struct TailscaleService {
    /// Whether to run in dry-run mode.
    dry_run: bool,
}

impl TailscaleService {
    /// Create a new `TailscaleService`.
    pub fn new() -> Self {
        Self { dry_run: false }
    }

    /// Enable dry-run mode (log commands but do not execute).
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Check if the `tailscaled` service is currently active.
    ///
    /// # Errors
    ///
    /// Returns an error if the service status cannot be determined.
    pub fn is_active(&self) -> Result<bool> {
        // TODO: Implement via systemctl or toride-runner.
        let _ = self.dry_run;
        Ok(false)
    }

    /// Start the `tailscaled` service.
    ///
    /// # Errors
    ///
    /// Returns an error if the service cannot be started.
    pub fn start(&self) -> Result<()> {
        // TODO: Implement via systemctl start tailscaled.
        Ok(())
    }

    /// Stop the `tailscaled` service.
    ///
    /// # Errors
    ///
    /// Returns an error if the service cannot be stopped.
    pub fn stop(&self) -> Result<()> {
        // TODO: Implement via systemctl stop tailscaled.
        Ok(())
    }

    /// Restart the `tailscaled` service.
    ///
    /// # Errors
    ///
    /// Returns an error if the service cannot be restarted.
    pub fn restart(&self) -> Result<()> {
        // TODO: Implement via systemctl restart tailscaled.
        Ok(())
    }

    /// Enable the `tailscaled` service to start on boot.
    ///
    /// # Errors
    ///
    /// Returns an error if the service cannot be enabled.
    pub fn enable(&self) -> Result<()> {
        // TODO: Implement via systemctl enable tailscaled.
        Ok(())
    }

    /// Disable the `tailscaled` service from starting on boot.
    ///
    /// # Errors
    ///
    /// Returns an error if the service cannot be disabled.
    pub fn disable(&self) -> Result<()> {
        // TODO: Implement via systemctl disable tailscaled.
        Ok(())
    }
}

impl Default for TailscaleService {
    fn default() -> Self {
        Self::new()
    }
}
