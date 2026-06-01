//! Service management for the toride-monitor daemon.
//!
//! Provides [`MonitorService`] for managing the monitoring daemon lifecycle:
//! start, stop, restart, status checks, and enable/disable on boot.

use crate::client::MonitorClient;
use crate::paths::MonitorPaths;
use crate::spec::MonitorSpec;
use crate::Result;

/// Manages the toride-monitor system service.
///
/// Wraps service management operations (systemd unit, sysvinit script, etc.)
/// for the monitoring daemon.
pub struct MonitorService<'a> {
    /// The underlying monitoring client.
    client: MonitorClient,
    /// Binary paths for system commands.
    _paths: &'a MonitorPaths,
}

impl<'a> MonitorService<'a> {
    /// Create a new `MonitorService` with the given client and paths.
    #[must_use]
    pub fn new(client: MonitorClient, paths: &'a MonitorPaths) -> Self {
        Self {
            client,
            _paths: paths,
        }
    }

    /// Start the monitoring service.
    ///
    /// Sets up logging rules and begins periodic monitoring according to the
    /// provided spec.
    ///
    /// # Errors
    ///
    /// Returns an error if logging setup fails.
    pub fn start(&self, spec: &MonitorSpec) -> Result<()> {
        tracing::info!("Starting toride-monitor service");
        self.client.setup_logging(&spec.logging_rules)?;
        Ok(())
    }

    /// Stop the monitoring service.
    ///
    /// Tears down logging rules and stops periodic monitoring.
    ///
    /// # Errors
    ///
    /// Returns an error if logging teardown fails.
    pub fn stop(&self) -> Result<()> {
        tracing::info!("Stopping toride-monitor service");
        self.client.teardown_logging()
    }

    /// Restart the monitoring service.
    ///
    /// Equivalent to `stop()` followed by `start()`.
    ///
    /// # Errors
    ///
    /// Returns an error if either stop or start fails.
    pub fn restart(&self, spec: &MonitorSpec) -> Result<()> {
        self.stop()?;
        self.start(spec)
    }

    /// Check whether the monitoring service is currently active.
    ///
    /// # Errors
    ///
    /// Returns an error if the check cannot be performed.
    pub fn is_active(&self) -> Result<bool> {
        // TODO: Check systemd unit status or PID file.
        Ok(false)
    }

    /// Run a single monitoring cycle.
    ///
    /// Takes a snapshot, detects anomalies, and dispatches alerts according
    /// to the provided spec. Intended to be called periodically by the
    /// daemon loop.
    ///
    /// # Errors
    ///
    /// Returns an error if the snapshot or detection fails.
    pub fn run_cycle(&self, spec: &MonitorSpec) -> Result<crate::report::AnomalyReport> {
        self.client.apply(spec)
    }
}
