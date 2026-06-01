//! systemd-journald management.
//!
//! Provides functions for interacting with systemd-journald for
//! audit log aggregation and querying.

use crate::{AuditPaths, Error, Result};

// ---------------------------------------------------------------------------
// JournaldManager
// ---------------------------------------------------------------------------

/// Manager for journald-based audit log operations.
///
/// Provides methods for querying journal entries related to the audit
/// subsystem and checking journald configuration.
pub struct JournaldManager<'a> {
    runner: &'a dyn toride_runner::Runner,
    paths: &'a AuditPaths,
}

impl<'a> JournaldManager<'a> {
    /// Create a new journald manager with the given runner and paths.
    pub fn new(runner: &'a dyn toride_runner::Runner, paths: &'a AuditPaths) -> Self {
        Self { runner, paths }
    }

    /// Query audit-related journal entries.
    ///
    /// Uses `journalctl` with audit-related filters.
    ///
    /// # Arguments
    ///
    /// * `since` - Optional time filter (e.g. "today", "1 hour ago").
    /// * `unit` - Optional systemd unit filter (e.g. "auditd.service").
    ///
    /// # Errors
    ///
    /// Returns [`Error::BinaryNotFound`] if `journalctl` is not available.
    pub fn query(&self, since: Option<&str>, unit: Option<&str>) -> Result<String> {
        let bin = which::which("journalctl")
            .map_err(|_| Error::BinaryNotFound("journalctl".to_owned()))?;

        let mut args: Vec<String> = Vec::new();

        if let Some(s) = since {
            args.push("--since".to_owned());
            args.push(s.to_owned());
        }

        if let Some(u) = unit {
            args.push("--unit".to_owned());
            args.push(u.to_owned());
        }

        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = self.runner.run_output(bin, &arg_refs)?;
        Ok(output)
    }

    /// Query journal entries for the audit daemon.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BinaryNotFound`] if `journalctl` is not available.
    pub fn query_auditd(&self) -> Result<String> {
        self.query(None, Some("auditd.service"))
    }

    /// Get disk usage of the journal.
    ///
    /// Runs `journalctl --disk-usage`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BinaryNotFound`] if `journalctl` is not available.
    pub fn disk_usage(&self) -> Result<String> {
        let bin = which::which("journalctl")
            .map_err(|_| Error::BinaryNotFound("journalctl".to_owned()))?;
        let output = self.runner.run_output(bin, &["--disk-usage"])?;
        Ok(output)
    }

    /// Vacuum journal entries older than the specified time.
    ///
    /// Runs `journalctl --vacuum-time=<time>`.
    ///
    /// # Arguments
    ///
    /// * `time` - Time specification (e.g. "7d", "2weeks").
    ///
    /// # Errors
    ///
    /// Returns [`Error::BinaryNotFound`] if `journalctl` is not available.
    /// Returns [`Error::CommandFailed`] if the vacuum fails.
    pub fn vacuum_time(&self, time: &str) -> Result<()> {
        let bin = which::which("journalctl")
            .map_err(|_| Error::BinaryNotFound("journalctl".to_owned()))?;
        self.runner
            .run_checked(bin, &[&format!("--vacuum-time={time}")])?;
        Ok(())
    }
}
