//! Client for executing update-related commands.
//!
//! [`UpdatesClient`] wraps a [`toride_runner::Runner`] and provides high-level
//! methods for checking, applying, configuring, and querying the status of
//! automatic security updates.

use tracing::info;

use crate::detect::PackageManager;
use crate::error::{Error, Result};
use crate::paths::UpdatePaths;
use crate::report::UpdateStatus;
use crate::spec::UpdateSpec;

// ---------------------------------------------------------------------------
// UpdatesClient
// ---------------------------------------------------------------------------

/// Client for interacting with the system's automatic update subsystem.
///
/// Owns a boxed [`toride_runner::Runner`] for command execution and resolved
/// [`UpdatePaths`] for locating configuration files.
///
/// # Construction
///
/// - [`UpdatesClient::new`] -- production defaults using `duct`.
/// - [`UpdatesClient::with_runner`] -- inject a custom runner for testing.
pub struct UpdatesClient {
    runner: Box<dyn toride_runner::Runner>,
    paths: UpdatePaths,
}

impl UpdatesClient {
    /// Create a new client with production defaults.
    ///
    /// Uses `duct` for command execution and auto-detects update paths.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BinaryNotFound`] if no supported package manager is
    /// detected on the system.
    pub fn new() -> Result<Self> {
        let pkg_mgr = crate::detect::detect_package_manager(); // from detect module
        let paths = UpdatePaths::detect();

        if pkg_mgr == PackageManager::Unknown {
            return Err(Error::PackageDetection(
                "neither apt-get nor dnf found on $PATH".into(),
            ));
        }

        Ok(Self {
            runner: Box::new(toride_runner::DuctRunner),
            paths,
        })
    }

    /// Create a client with a custom runner (for testing).
    pub fn with_runner(runner: Box<dyn toride_runner::Runner>) -> Self {
        Self {
            runner,
            paths: UpdatePaths::new(),
        }
    }

    /// Create a client with both a custom runner and explicit paths.
    pub fn with_runner_and_paths(
        runner: Box<dyn toride_runner::Runner>,
        paths: UpdatePaths,
    ) -> Self {
        Self { runner, paths }
    }

    // -----------------------------------------------------------------------
    // Operations
    // -----------------------------------------------------------------------

    /// Check for available updates and return the counts.
    ///
    /// On APT systems, runs `apt-check`. On DNF systems, runs
    /// `dnf check-update`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the command fails.
    pub fn check_updates(&self) -> Result<(usize, usize)> {
        let pkg_mgr = crate::detect::detect_package_manager();
        match pkg_mgr {
            PackageManager::Apt => self.check_updates_apt(),
            PackageManager::Dnf => self.check_updates_dnf(),
            PackageManager::Unknown => Err(Error::PackageDetection(
                "no supported package manager".into(),
            )),
        }
    }

    /// Apply pending updates now.
    ///
    /// On APT systems, runs `unattended-upgrades`. On DNF systems, runs
    /// `dnf-automatic`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the update command fails.
    pub fn apply_updates(&self) -> Result<()> {
        info!("Applying pending updates");
        // TODO: Implement with actual command execution.
        Ok(())
    }

    /// Configure automatic updates according to the given spec.
    ///
    /// This writes the appropriate config files and enables/disables the
    /// update service.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ConfigWrite`] if any config file cannot be written.
    pub fn configure(&self, spec: &UpdateSpec) -> Result<()> {
        info!("Configuring automatic updates");
        // TODO: Render spec, backup existing configs, write new configs.
        let _ = spec;
        Ok(())
    }

    /// Query the current update status.
    ///
    /// Returns an [`UpdateStatus`] reflecting the current state of automatic
    /// updates on this host.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the status query fails.
    pub fn status(&self) -> Result<UpdateStatus> {
        info!("Querying update status");
        // TODO: Implement with actual command execution and parsing.
        Ok(UpdateStatus::empty())
    }

    // -----------------------------------------------------------------------
    // Backend helpers
    // -----------------------------------------------------------------------

    fn check_updates_apt(&self) -> Result<(usize, usize)> {
        use toride_runner::CommandSpec;
        let spec = CommandSpec::new("/usr/lib/update-notifier/apt-check");
        let output = self
            .runner
            .run_checked(&spec)
            .map_err(|e| Error::CommandFailed(format!("apt-check failed: {e}")))?;
        crate::parse::parse_apt_check(&output.stderr)
    }

    fn check_updates_dnf(&self) -> Result<(usize, usize)> {
        use toride_runner::CommandSpec;
        let spec = CommandSpec::new("dnf").args(["check-update", "--security"]);
        let output = self
            .runner
            .run_checked(&spec)
            .map_err(|e| Error::CommandFailed(format!("dnf check-update failed: {e}")))?;
        crate::parse::parse_dnf_check(&output.stdout)
    }
}
