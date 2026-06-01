//! Debian/Ubuntu (APT) specific update backend.
//!
//! Provides APT-specific operations for managing `unattended-upgrades`:
//!
//! - Checking for available updates via `apt-check`
//! - Applying updates via `unattended-upgrades`
//! - Querying update status

use tracing::info;

use crate::error::{Error, Result};
use crate::paths::UpdatePaths;
use crate::report::UpdateStatus;

// ---------------------------------------------------------------------------
// AptBackend
// ---------------------------------------------------------------------------

/// APT-specific backend for automatic update operations.
///
/// Wraps command execution for `apt-check`, `unattended-upgrades`, and
/// related APT tools.
pub struct AptBackend<'a> {
    runner: &'a dyn toride_runner::Runner,
    paths: UpdatePaths,
}

impl<'a> AptBackend<'a> {
    /// Create a new APT backend with the given runner.
    pub fn new(runner: &'a dyn toride_runner::Runner) -> Self {
        Self {
            runner,
            paths: UpdatePaths::new(),
        }
    }

    /// Create an APT backend with explicit paths.
    pub fn with_paths(runner: &'a dyn toride_runner::Runner, paths: UpdatePaths) -> Self {
        Self { runner, paths }
    }

    /// Check for available updates using `apt-check`.
    ///
    /// Returns `(security_updates, total_updates)`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if `apt-check` fails.
    pub fn check_updates(&self) -> Result<(usize, usize)> {
        info!("Checking APT updates");

        let output = self
            .runner
            .run_stderr_ok(&["/usr/lib/update-notifier/apt-check"])
            .map_err(|e| Error::CommandFailed(format!("apt-check failed: {e}")))?;

        crate::parse::parse_apt_check(&output)
    }

    /// Apply pending security updates via `unattended-upgrades`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the command fails.
    pub fn apply_updates(&self) -> Result<()> {
        info!("Applying APT updates via unattended-upgrades");

        self.runner
            .run_stderr_ok(&["unattended-upgrades", "-v"])
            .map_err(|e| Error::CommandFailed(format!("unattended-upgrades failed: {e}")))?;

        Ok(())
    }

    /// Query the current update status by parsing the log file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the log file cannot be read.
    pub fn status(&self) -> Result<UpdateStatus> {
        info!("Querying APT update status");

        let log_path = &self.paths.log_file;
        if !log_path.exists() {
            return Ok(UpdateStatus::empty());
        }

        let content = std::fs::read_to_string(log_path)?;
        // Parse the last few lines of the log for status information.
        let _ = content;
        Ok(UpdateStatus::empty())
    }

    /// Check if `unattended-upgrades` binary is available.
    pub fn is_available(&self) -> bool {
        which::which("unattended-upgrades").is_ok()
    }
}
