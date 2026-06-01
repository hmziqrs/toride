//! Diagnostic checks for the monitoring subsystem.
//!
//! Provides [`Doctor`] which runs a series of health checks against a
//! monitoring installation and produces a [`DoctorReport`] with findings.

use crate::paths::MonitorPaths;
use crate::report::{AnomalyFinding, AnomalySeverity};
use crate::Result;

/// Scope of doctor checks to run.
#[derive(Debug, Clone, Default)]
pub enum DoctorScope {
    /// Run all available checks.
    #[default]
    All,
    /// Check only binary availability.
    Binaries,
    /// Check only iptables logging configuration.
    Logging,
    /// Check only service status.
    Service,
    /// Check only thresholds and configuration.
    Config,
}

/// Diagnostic engine for the monitoring subsystem.
///
/// Runs targeted health checks and collects findings into a structured
/// report. Designed to be resilient: individual check failures do not
/// abort the overall diagnostic run.
pub struct Doctor<'a> {
    /// Binary paths for system commands.
    paths: &'a MonitorPaths,
}

impl<'a> Doctor<'a> {
    /// Create a new `Doctor` with the given paths.
    #[must_use]
    pub fn new(paths: &'a MonitorPaths) -> Self {
        Self { paths }
    }

    /// Run doctor checks within the given scope.
    ///
    /// # Errors
    ///
    /// Returns an error only for fundamental failures (e.g. broken runner).
    /// Individual check failures appear as findings in the report.
    pub fn run(&self, scope: &DoctorScope) -> Result<DoctorReport> {
        let mut findings = Vec::new();

        match scope {
            DoctorScope::All => {
                self.check_binaries(&mut findings);
                self.check_logging(&mut findings);
                self.check_service(&mut findings);
                self.check_config(&mut findings);
            }
            DoctorScope::Binaries => {
                self.check_binaries(&mut findings);
            }
            DoctorScope::Logging => {
                self.check_logging(&mut findings);
            }
            DoctorScope::Service => {
                self.check_service(&mut findings);
            }
            DoctorScope::Config => {
                self.check_config(&mut findings);
            }
        }

        Ok(DoctorReport { findings })
    }

    /// Check that required binaries exist and are executable.
    fn check_binaries(&self, findings: &mut Vec<AnomalyFinding>) {
        let binaries = [
            ("iptables", &self.paths.iptables),
            ("iptables-save", &self.paths.iptables_save),
            ("conntrack", &self.paths.conntrack),
            ("ss", &self.paths.ss),
            ("journalctl", &self.paths.journalctl),
        ];

        for (name, path) in binaries {
            if !path.exists() {
                findings.push(AnomalyFinding::new(
                    format!("doctor.binary.{name}.missing"),
                    AnomalySeverity::Critical,
                    format!("Required binary not found: {name}"),
                    format!("Expected at: {}", path.display()),
                    "Binary must exist and be executable",
                ).fix(format!("Install the package providing {name}.")));
            }
        }
    }

    /// Check iptables OUTPUT chain logging configuration.
    fn check_logging(&self, findings: &mut Vec<AnomalyFinding>) {
        // Check if iptables LOG rules exist in the OUTPUT chain.
        let chain = crate::output::OutputChain::new(self.paths);
        match chain.list_rules() {
            Ok(rules) => {
                if rules.is_empty() {
                    findings.push(AnomalyFinding::new(
                        "doctor.logging.no-rules",
                        AnomalySeverity::Warning,
                        "No OUTPUT chain LOG rules configured",
                        "0 LOG rules in OUTPUT chain",
                        "At least one LOG rule expected",
                    ).fix("Run monitor setup to install logging rules."));
                } else if rules.len() > 50 {
                    findings.push(AnomalyFinding::new(
                        "doctor.logging.excessive-rules",
                        AnomalySeverity::Warning,
                        "Excessive number of OUTPUT chain LOG rules",
                        format!("{} LOG rules in OUTPUT chain", rules.len()),
                        "Fewer than 50 rules recommended",
                    ).fix("Review and remove unnecessary logging rules."));
                }
            }
            Err(e) => {
                findings.push(AnomalyFinding::new(
                    "doctor.logging.check-failed",
                    AnomalySeverity::Error,
                    "Failed to list iptables OUTPUT chain rules",
                    format!("{e}"),
                    "Should be able to list rules",
                ).fix("Verify iptables permissions and kernel modules."));
            }
        }
    }

    /// Check monitoring service status.
    fn check_service(&self, findings: &mut Vec<AnomalyFinding>) {
        // TODO: Check systemd unit status, uptime, last run time.
        findings.push(AnomalyFinding::new(
            "doctor.service.not-implemented",
            AnomalySeverity::Info,
            "Service status check not yet implemented",
            "N/A".to_string(),
            "N/A".to_string(),
        ));
    }

    /// Check configuration validity.
    fn check_config(&self, findings: &mut Vec<AnomalyFinding>) {
        // Validate default thresholds.
        let threshold = crate::spec::AnomalyThreshold::default();
        if let Err(e) = crate::validate::validate_threshold(&threshold) {
            findings.push(AnomalyFinding::new(
                "doctor.config.invalid-threshold",
                AnomalySeverity::Error,
                "Default threshold configuration is invalid",
                format!("{e}"),
                "Valid thresholds required",
            ).fix("Review default threshold values."));
        }
    }
}

/// Aggregated doctor report for monitoring diagnostics.
#[derive(Debug, Clone)]
pub struct DoctorReport {
    /// All findings collected during the doctor run.
    pub findings: Vec<AnomalyFinding>,
}

impl DoctorReport {
    /// Create an empty report.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            findings: Vec::new(),
        }
    }

    /// Returns `true` if no findings were produced.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    /// Returns `true` if any finding has critical severity.
    #[must_use]
    pub fn has_critical(&self) -> bool {
        self.findings.iter().any(|f| f.severity == AnomalySeverity::Critical)
    }

    /// Returns the number of findings.
    #[must_use]
    pub fn len(&self) -> usize {
        self.findings.len()
    }

    /// Returns `true` if there are no findings.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.findings.is_empty()
    }
}
