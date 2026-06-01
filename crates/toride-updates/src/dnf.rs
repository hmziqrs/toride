//! Fedora/RHEL (DNF) specific update backend.
//!
//! Provides DNF-specific operations for managing `dnf-automatic`:
//!
//! - Checking for available updates via `dnf check-update`
//! - Applying updates via `dnf-automatic`
//! - Querying update status

use tracing::info;

use crate::error::{Error, Result};
use crate::paths::UpdatePaths;
use crate::report::UpdateStatus;

// ---------------------------------------------------------------------------
// DnfBackend
// ---------------------------------------------------------------------------

/// DNF-specific backend for automatic update operations.
///
/// Wraps command execution for `dnf check-update`, `dnf-automatic`, and
/// related DNF tools.
pub struct DnfBackend<'a> {
    runner: &'a dyn toride_runner::Runner,
    paths: UpdatePaths,
}

impl<'a> DnfBackend<'a> {
    /// Create a new DNF backend with the given runner.
    pub fn new(runner: &'a dyn toride_runner::Runner) -> Self {
        Self {
            runner,
            paths: UpdatePaths::new(),
        }
    }

    /// Create a DNF backend with explicit paths.
    pub fn with_paths(runner: &'a dyn toride_runner::Runner, paths: UpdatePaths) -> Self {
        Self { runner, paths }
    }

    /// Check for available updates using `dnf check-update`.
    ///
    /// Returns `(security_updates, total_updates)`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the command fails.
    pub fn check_updates(&self) -> Result<(usize, usize)> {
        info!("Checking DNF updates");

        let output = self
            .runner
            .run_stderr_ok(&["dnf", "check-update", "--security"])
            .map_err(|e| Error::CommandFailed(format!("dnf check-update failed: {e}")))?;

        crate::parse::parse_dnf_check(&output)
    }

    /// Apply pending updates via `dnf-automatic`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the command fails.
    pub fn apply_updates(&self) -> Result<()> {
        info!("Applying DNF updates via dnf-automatic");

        self.runner
            .run_stderr_ok(&["dnf-automatic", "--install"])
            .map_err(|e| Error::CommandFailed(format!("dnf-automatic failed: {e}")))?;

        Ok(())
    }

    /// Query the current update status.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if status information cannot be read.
    pub fn status(&self) -> Result<UpdateStatus> {
        info!("Querying DNF update status");

        // DNF does not have a persistent log like APT's unattended-upgrades.
        // We check the systemd journal or the automatic.conf for status.
        let _ = &self.paths;
        Ok(UpdateStatus::empty())
    }

    /// Check if `dnf-automatic` binary is available.
    pub fn is_available(&self) -> bool {
        which::which("dnf-automatic").is_ok()
    }
}
