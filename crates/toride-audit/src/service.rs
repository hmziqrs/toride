//! Service management for audit-related daemons.
//!
//! Provides start/stop/restart/reload operations for `auditd` and related
//! services using systemctl commands through the runner.

use toride_runner::CommandSpec;

use crate::{AuditPaths, Result};

// ---------------------------------------------------------------------------
// AuditServiceManager
// ---------------------------------------------------------------------------

/// Manager for audit-related system services.
///
/// Provides lifecycle operations for the audit daemon and related services
/// (rsyslog, journald) by issuing `systemctl` commands through the runner.
pub struct AuditServiceManager<'a> {
    runner: &'a dyn toride_runner::Runner,
    paths: &'a AuditPaths,
}

impl<'a> AuditServiceManager<'a> {
    /// Create a new service manager with the given runner.
    pub fn new(runner: &'a dyn toride_runner::Runner, paths: &'a AuditPaths) -> Self {
        Self { runner, paths }
    }

    /// Check if the auditd service is active.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::CommandFailed`] if the check cannot be performed.
    pub fn is_auditd_active(&self) -> Result<bool> {
        let spec = CommandSpec::new("systemctl").args(["is-active", "auditd"]);
        let output = self.runner.run(&spec)?;
        Ok(output.success)
    }

    /// Restart the auditd service.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::CommandFailed`] if the restart fails.
    pub fn restart_auditd(&self) -> Result<()> {
        let spec = CommandSpec::new("systemctl").args(["restart", "auditd"]);
        let output = self.runner.run(&spec)?;
        if output.success {
            Ok(())
        } else {
            Err(crate::Error::CommandFailed(format!(
                "systemctl restart auditd failed: {}",
                output.stderr.trim()
            )))
        }
    }

    /// Reload auditd rules without restarting the service.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::CommandFailed`] if the reload fails.
    pub fn reload_auditd_rules(&self) -> Result<()> {
        which::which("auditctl")
            .map_err(|_| crate::Error::BinaryNotFound("auditctl".to_owned()))?;
        let spec = CommandSpec::new("auditctl")
            .arg("-R")
            .arg(self.paths.rules_path("audit").to_str().unwrap_or_default());
        self.runner.run_checked(&spec)?;
        Ok(())
    }

    /// Enable and start the auditd service.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::CommandFailed`] if the operation fails.
    pub fn enable_and_start_auditd(&self) -> Result<()> {
        let enable_spec = CommandSpec::new("systemctl").args(["enable", "auditd"]);
        let output = self.runner.run(&enable_spec)?;
        if !output.success {
            return Err(crate::Error::CommandFailed(format!(
                "systemctl enable auditd failed: {}",
                output.stderr.trim()
            )));
        }
        let start_spec = CommandSpec::new("systemctl").args(["start", "auditd"]);
        let output = self.runner.run(&start_spec)?;
        if !output.success {
            return Err(crate::Error::CommandFailed(format!(
                "systemctl start auditd failed: {}",
                output.stderr.trim()
            )));
        }
        Ok(())
    }

    /// Check if rsyslog is active.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::CommandFailed`] if the check cannot be performed.
    pub fn is_rsyslog_active(&self) -> Result<bool> {
        let spec = CommandSpec::new("systemctl").args(["is-active", "rsyslog"]);
        let output = self.runner.run(&spec)?;
        Ok(output.success)
    }

    /// Restart the rsyslog service.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::CommandFailed`] if the restart fails.
    pub fn restart_rsyslog(&self) -> Result<()> {
        let spec = CommandSpec::new("systemctl").args(["restart", "rsyslog"]);
        let output = self.runner.run(&spec)?;
        if output.success {
            Ok(())
        } else {
            Err(crate::Error::CommandFailed(format!(
                "systemctl restart rsyslog failed: {}",
                output.stderr.trim()
            )))
        }
    }
}
