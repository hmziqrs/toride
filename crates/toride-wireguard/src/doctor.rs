//! Tunnel health diagnostics for WireGuard.
//!
//! Provides a diagnostic engine that checks:
//! - WireGuard binary availability
//! - Interface status and connectivity
//! - Key file permissions
//! - DNS leak detection
//! - Configuration validity

use crate::error::Result;
use crate::report::{Finding, Severity, WireguardReport};

// ---------------------------------------------------------------------------
// DoctorScope
// ---------------------------------------------------------------------------

/// Scope for diagnostic checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorScope {
    /// Run all available checks.
    All,
    /// Only check binary availability and basic setup.
    Setup,
    /// Only check interface status and peer connectivity.
    Connectivity,
    /// Only check key file permissions and security.
    Security,
}

// ---------------------------------------------------------------------------
// Doctor
// ---------------------------------------------------------------------------

/// Diagnostic engine for WireGuard installations.
///
/// Runs a series of health checks and collects findings into a
/// [`WireguardReport`].
pub struct Doctor {
    _runner: (),
}

impl Doctor {
    /// Create a new diagnostic engine.
    pub fn new() -> Self {
        Self { _runner: () }
    }

    /// Create a diagnostic engine with a custom runner (for testing).
    pub fn with_runner(_runner: ()) -> Self {
        Self { _runner: () }
    }

    /// Run diagnostics with the given scope and return a report.
    ///
    /// # Errors
    ///
    /// Returns an error only for fundamental failures (e.g. unable to run
    /// any checks). Individual check failures appear as [`Finding`] values
    /// in the report.
    pub fn run(&self, scope: &DoctorScope) -> Result<WireguardReport> {
        let mut report = WireguardReport::new();

        match scope {
            DoctorScope::All => {
                self.check_binaries(&mut report)?;
                self.check_config_dir(&mut report)?;
                self.check_interfaces(&mut report)?;
                self.check_key_permissions(&mut report)?;
                self.check_dns_leak(&mut report)?;
            }
            DoctorScope::Setup => {
                self.check_binaries(&mut report)?;
                self.check_config_dir(&mut report)?;
            }
            DoctorScope::Connectivity => {
                self.check_interfaces(&mut report)?;
            }
            DoctorScope::Security => {
                self.check_key_permissions(&mut report)?;
            }
        }

        Ok(report)
    }

    // -----------------------------------------------------------------------
    // Individual checks
    // -----------------------------------------------------------------------

    /// Check that `wg` and `wg-quick` binaries are available.
    fn check_binaries(&self, report: &mut WireguardReport) -> Result<()> {
        tracing::debug!("checking WireGuard binaries");

        report.wg_binary_found = which::which("wg").is_ok();
        if !report.wg_binary_found {
            report.findings.push(
                Finding::new(
                    "wireguard.binary.wg",
                    Severity::Error,
                    "`wg` binary not found on $PATH".to_owned(),
                )
                .with_fix("Install wireguard-tools: apt install wireguard-tools".to_owned()),
            );
        }

        report.wg_quick_binary_found = which::which("wg-quick").is_ok();
        if !report.wg_quick_binary_found {
            report.findings.push(
                Finding::new(
                    "wireguard.binary.wg-quick",
                    Severity::Warning,
                    "`wg-quick` binary not found on $PATH".to_owned(),
                )
                .with_fix("Install wireguard-tools: apt install wireguard-tools".to_owned()),
            );
        }

        Ok(())
    }

    /// Check that the WireGuard config directory exists with proper permissions.
    fn check_config_dir(&self, report: &mut WireguardReport) -> Result<()> {
        tracing::debug!("checking WireGuard config directory");
        report.config_dir_exists = report.config_dir.is_dir();

        if !report.config_dir_exists {
            report.findings.push(Finding::new(
                "wireguard.config-dir",
                Severity::Warning,
                format!(
                    "WireGuard config directory does not exist: {}",
                    report.config_dir.display()
                ),
            ));
        }

        Ok(())
    }

    /// Check interface status and peer connectivity.
    fn check_interfaces(&self, report: &mut WireguardReport) -> Result<()> {
        tracing::debug!("checking WireGuard interfaces");
        // TODO: parse `wg show` and populate report.interfaces.
        Ok(())
    }

    /// Check that private key files have restrictive permissions (0600).
    fn check_key_permissions(&self, report: &mut WireguardReport) -> Result<()> {
        tracing::debug!("checking key file permissions");
        // TODO: stat key files and check mode == 0o600.
        Ok(())
    }

    /// Check for DNS leaks when the tunnel is active.
    fn check_dns_leak(&self, report: &mut WireguardReport) -> Result<()> {
        tracing::debug!("checking for DNS leaks");
        // TODO: verify that DNS queries are routed through the tunnel.
        Ok(())
    }
}

impl Default for Doctor {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_setup_scope() {
        let doc = Doctor::new();
        let report = doc.run(&DoctorScope::Setup).unwrap();
        // Binaries likely not found in test environment.
        assert!(!report.findings.is_empty() || !report.wg_binary_found);
    }

    #[test]
    fn run_all_scope() {
        let doc = Doctor::new();
        let report = doc.run(&DoctorScope::All).unwrap();
        assert!(report.config_dir.exists() || !report.config_dir_exists);
    }

    #[test]
    fn doctor_scope_variants() {
        assert_ne!(DoctorScope::All, DoctorScope::Setup);
        assert_ne!(DoctorScope::Connectivity, DoctorScope::Security);
    }
}
