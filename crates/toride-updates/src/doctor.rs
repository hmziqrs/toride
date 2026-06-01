//! Doctor checks for the automatic update subsystem.
//!
//! Runs a battery of diagnostic checks to verify that automatic security
//! updates are correctly configured and functioning:
//!
//! - Binary availability (unattended-upgrades, dnf-automatic)
//! - Service active status
//! - Auto-updates enabled
//! - Schedule configured
//! - Stale last-run detection
//! - Config directory permissions

use tracing::info;

use crate::detect::PackageManager;
use crate::error::Result;
use crate::paths::UpdatePaths;
use crate::report;

// ---------------------------------------------------------------------------
// Doctor
// ---------------------------------------------------------------------------

/// Diagnostic engine for the updates subsystem.
///
/// Runs checks and returns a list of [`toride_diagnostic_types::Finding`]
/// values indicating any issues detected.
pub struct Doctor<'a> {
    _runner: &'a dyn toride_runner::Runner,
    paths: UpdatePaths,
    pkg_mgr: PackageManager,
}

impl<'a> Doctor<'a> {
    /// Create a new doctor instance with the given runner.
    pub fn new(runner: &'a dyn toride_runner::Runner) -> Self {
        Self {
            _runner: runner,
            paths: UpdatePaths::detect(),
            pkg_mgr: crate::detect::detect_package_manager(),
        }
    }

    /// Run all diagnostic checks and return the findings.
    ///
    /// # Errors
    ///
    /// Returns an error only for fundamental failures (e.g. runner broken).
    /// Individual check failures appear as findings in the returned list.
    pub fn run(&self) -> Result<Vec<toride_diagnostic_types::Finding>> {
        let mut findings = Vec::new();

        self.check_binary_available(&mut findings);
        self.check_service_active(&mut findings);
        self.check_auto_updates_enabled(&mut findings);
        self.check_schedule_configured(&mut findings);
        self.check_last_run_fresh(&mut findings);
        self.check_config_dir_permissions(&mut findings);

        Ok(findings)
    }

    // -----------------------------------------------------------------------
    // Individual checks
    // -----------------------------------------------------------------------

    /// Check: binary.unattended-upgrades.missing / binary.dnf-automatic.missing
    fn check_binary_available(&self, findings: &mut Vec<toride_diagnostic_types::Finding>) {
        info!("Checking binary availability");

        let binary = match self.pkg_mgr {
            PackageManager::Apt => "unattended-upgrades",
            PackageManager::Dnf => "dnf-automatic",
            PackageManager::Unknown => {
                findings.push(report::finding_binary_missing("package-manager"));
                return;
            }
        };

        if which::which(binary).is_err() {
            findings.push(report::finding_binary_missing(binary));
        }
    }

    /// Check: service.unattended-upgrades.inactive / service.dnf-automatic.inactive
    fn check_service_active(&self, _findings: &mut Vec<toride_diagnostic_types::Finding>) {
        info!("Checking service active status");

        let service = match self.pkg_mgr {
            PackageManager::Apt => "unattended-upgrades",
            PackageManager::Dnf => "dnf-automatic.timer",
            PackageManager::Unknown => return,
        };

        // TODO: Query systemd via toride_service.
        // For now, just emit a placeholder check.
        let _ = service;
    }

    /// Check: config.auto-updates.disabled
    fn check_auto_updates_enabled(&self, _findings: &mut Vec<toride_diagnostic_types::Finding>) {
        info!("Checking auto-updates enabled");

        // TODO: Parse 20auto-upgrades (APT) or automatic.conf (DNF).
        let _ = &self.paths;
    }

    /// Check: config.schedule.missing
    fn check_schedule_configured(&self, _findings: &mut Vec<toride_diagnostic_types::Finding>) {
        info!("Checking schedule configuration");

        // TODO: Check for systemd timer or cron job.
        let _ = &self.paths;
    }

    /// Check: schedule.stale-last-run
    fn check_last_run_fresh(&self, _findings: &mut Vec<toride_diagnostic_types::Finding>) {
        info!("Checking last run freshness");

        // TODO: Parse log file and compare last run timestamp with schedule.
        let _ = &self.paths;
    }

    /// Check: permission.config-dir-world-writable
    fn check_config_dir_permissions(&self, findings: &mut Vec<toride_diagnostic_types::Finding>) {
        info!("Checking config directory permissions");

        let dir = match self.pkg_mgr {
            PackageManager::Apt => &self.paths.apt_conf_d,
            PackageManager::Dnf => &self.paths.dnf_conf_d,
            PackageManager::Unknown => return,
        };

        if let Ok(metadata) = std::fs::metadata(dir) {
            // On Unix, check if the "other" write bit is set.
            #[expect(clippy::unnecessary_cast, reason = "mode_bits only on Unix")]
            let mode = metadata.permissions().mode() as u32;
            if mode & 0o002 != 0 {
                findings.push(report::finding_config_dir_world_writable(
                    &dir.display().to_string(),
                ));
            }
        }
    }
}

// Unix-specific imports for permission checking.
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
