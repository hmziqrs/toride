//! Diagnostic engine for backup installations.
//!
//! [`Doctor`] runs structured diagnostic checks across the backup
//! configuration and returns a [`DoctorReport`] containing typed findings
//! with severity levels, descriptions, and suggested fixes.
//!
//! # Categories
//!
//! | Scope | What it checks |
//! |-------|---------------|
//! | `Binary` | restic/borg binaries exist and are functional |
//! | `Repository` | repository exists, is accessible, has recent snapshots |
//! | `Staleness` | backups are not stale (run within expected schedule) |
//! | `Integrity` | last `check` passed without errors |
//! | `Encryption` | encryption is enabled, password command works |
//! | `Schedule` | systemd timers / cron entries are installed and active |
//! | `Retention` | retention policy is configured and has been applied |
//! | `Space` | repository / target filesystem has sufficient free space |
//!
//! # Example
//!
//! ```ignore
//! use toride_backup::doctor::{Doctor, DoctorScope};
//!
//! let doctor = Doctor::new();
//! let report = doctor.run(&DoctorScope::All)?;
//! if report.has_errors() {
//!     for f in &report.findings {
//!         eprintln!("[{}] {}", f.severity, f.title);
//!     }
//! }
//! ```

use crate::{Error, Result};

// ---------------------------------------------------------------------------
// Severity
// ---------------------------------------------------------------------------

/// Diagnostic severity level for doctor findings.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum Severity {
    /// No issue detected.
    Ok,
    /// Informational note; no action required.
    Info,
    /// Non-critical issue that may cause problems later.
    Warning,
    /// An error that should be addressed before proceeding.
    Error,
    /// A critical problem that blocks normal operation.
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok => write!(f, "OK"),
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARNING"),
            Self::Error => write!(f, "ERROR"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

// ---------------------------------------------------------------------------
// Finding
// ---------------------------------------------------------------------------

/// A single structured finding produced by the doctor.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Machine-readable dot-separated identifier.
    pub id: String,
    /// How severe this finding is.
    pub severity: Severity,
    /// Short human-readable title (one line).
    pub title: String,
    /// Longer description of the finding.
    pub detail: String,
    /// Suggested remediation action, if applicable.
    pub fix: Option<String>,
}

impl Finding {
    /// Create a new finding with the mandatory fields.
    pub fn new(
        id: impl Into<String>,
        severity: Severity,
        title: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            severity,
            title: title.into(),
            detail: String::new(),
            fix: None,
        }
    }

    /// Attach a longer description.
    #[must_use]
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }

    /// Attach a suggested fix.
    #[must_use]
    pub fn fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(fix.into());
        self
    }
}

// ---------------------------------------------------------------------------
// DoctorScope
// ---------------------------------------------------------------------------

/// Selects which diagnostic category (or categories) to run.
#[derive(Debug, Clone)]
pub enum DoctorScope {
    /// Run all diagnostic categories.
    All,
    /// Check that required binaries (restic/borg) exist.
    Binary,
    /// Check repository accessibility and state.
    Repository(String),
    /// Check that backups are not stale.
    Staleness(String),
    /// Check repository integrity.
    Integrity(String),
    /// Check encryption is enabled and functional.
    Encryption(String),
    /// Check schedules are installed and active.
    Schedule(String),
    /// Check retention policies are applied.
    Retention(String),
    /// Check filesystem has sufficient free space.
    Space(String),
}

impl DoctorScope {
    /// Return all individual scope categories (excluding `All`).
    pub fn all_categories() -> Vec<DoctorScope> {
        vec![
            DoctorScope::Binary,
        ]
    }
}

// ---------------------------------------------------------------------------
// DoctorReport
// ---------------------------------------------------------------------------

/// Aggregated doctor report containing all findings from a diagnostic run.
#[derive(Debug, Clone)]
pub struct DoctorReport {
    /// All findings collected during the doctor run.
    pub findings: Vec<Finding>,
}

impl DoctorReport {
    /// Create an empty report.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            findings: Vec::new(),
        }
    }

    /// Add a finding to the report.
    pub fn push(&mut self, finding: Finding) {
        self.findings.push(finding);
    }

    /// Returns the number of findings.
    #[must_use]
    pub fn len(&self) -> usize {
        self.findings.len()
    }

    /// Returns `true` if this report contains no findings.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.findings.is_empty()
    }

    /// Returns `true` if any finding has severity [`Severity::Error`] or higher.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|f| f.severity >= Severity::Error)
    }

    /// Returns `true` if any finding has severity [`Severity::Critical`].
    #[must_use]
    pub fn has_critical(&self) -> bool {
        self.findings.iter().any(|f| f.severity == Severity::Critical)
    }
}

// ---------------------------------------------------------------------------
// Doctor
// ---------------------------------------------------------------------------

/// Diagnostic engine that runs structured checks against backup
/// configurations and repositories.
pub struct Doctor;

impl Doctor {
    /// Create a new diagnostic engine.
    pub fn new() -> Self {
        Self
    }

    /// Run the selected diagnostic scope and return a complete report.
    ///
    /// # Errors
    ///
    /// Returns an error only if a fundamental failure occurs. Individual
    /// check failures appear as findings in the report.
    pub fn run(&self, scope: &DoctorScope) -> Result<DoctorReport> {
        let mut report = DoctorReport::empty();

        match scope {
            DoctorScope::All => {
                report.findings.extend(self.check_binaries());
                // TODO: add per-job checks when specs are provided.
            }
            DoctorScope::Binary => {
                report.findings.extend(self.check_binaries());
            }
            DoctorScope::Repository(name) => {
                report.findings.extend(self.check_repository(name));
            }
            DoctorScope::Staleness(name) => {
                report.findings.extend(self.check_staleness(name));
            }
            DoctorScope::Integrity(name) => {
                report.findings.extend(self.check_integrity(name));
            }
            DoctorScope::Encryption(name) => {
                report.findings.extend(self.check_encryption(name));
            }
            DoctorScope::Schedule(name) => {
                report.findings.extend(self.check_schedule(name));
            }
            DoctorScope::Retention(name) => {
                report.findings.extend(self.check_retention(name));
            }
            DoctorScope::Space(name) => {
                report.findings.extend(self.check_space(name));
            }
        }

        Ok(report)
    }

    // =======================================================================
    // Binary checks
    // =======================================================================

    /// Check that at least one backup binary (restic or borg) is available.
    fn check_binaries(&self) -> Vec<Finding> {
        let mut findings = Vec::new();

        let restic_available = which::which("restic").is_ok();
        let borg_available = which::which("borg").is_ok();

        if restic_available {
            findings.push(Finding::new(
                "binary.restic.found",
                Severity::Ok,
                "restic binary found on $PATH",
            ));
        } else {
            findings.push(
                Finding::new(
                    "binary.restic.missing",
                    Severity::Info,
                    "restic binary not found",
                )
                .detail(
                    "The restic binary is not on $PATH. Restic backups will \
                     not be available.",
                )
                .fix("Install restic: apt install restic (Debian/Ubuntu)."),
            );
        }

        if borg_available {
            findings.push(Finding::new(
                "binary.borg.found",
                Severity::Ok,
                "borg binary found on $PATH",
            ));
        } else {
            findings.push(
                Finding::new(
                    "binary.borg.missing",
                    Severity::Info,
                    "borg binary not found",
                )
                .detail(
                    "The borg binary is not on $PATH. Borg backups will \
                     not be available.",
                )
                .fix("Install borg: apt install borgbackup (Debian/Ubuntu)."),
            );
        }

        if !restic_available && !borg_available {
            findings.push(
                Finding::new(
                    "binary.none-available",
                    Severity::Critical,
                    "No backup binary available",
                )
                .detail(
                    "Neither restic nor borg was found on $PATH. At least \
                     one backup tool must be installed.",
                )
                .fix("Install restic or borg backup."),
            );
        }

        findings
    }

    // =======================================================================
    // Repository checks (skeleton)
    // =======================================================================

    fn check_repository(&self, name: &str) -> Vec<Finding> {
        // TODO: check repository accessibility.
        vec![Finding::new(
            "repository.check",
            Severity::Info,
            format!("Repository check for '{name}' not yet implemented"),
        )]
    }

    fn check_staleness(&self, name: &str) -> Vec<Finding> {
        // TODO: check last backup timestamp against schedule.
        vec![Finding::new(
            "staleness.check",
            Severity::Info,
            format!("Staleness check for '{name}' not yet implemented"),
        )]
    }

    fn check_integrity(&self, name: &str) -> Vec<Finding> {
        // TODO: run restic check / borg check.
        vec![Finding::new(
            "integrity.check",
            Severity::Info,
            format!("Integrity check for '{name}' not yet implemented"),
        )]
    }

    fn check_encryption(&self, name: &str) -> Vec<Finding> {
        // TODO: verify encryption is enabled and password command works.
        vec![Finding::new(
            "encryption.check",
            Severity::Info,
            format!("Encryption check for '{name}' not yet implemented"),
        )]
    }

    fn check_schedule(&self, name: &str) -> Vec<Finding> {
        // TODO: check systemd timer / cron entry.
        vec![Finding::new(
            "schedule.check",
            Severity::Info,
            format!("Schedule check for '{name}' not yet implemented"),
        )]
    }

    fn check_retention(&self, name: &str) -> Vec<Finding> {
        // TODO: verify retention policy is applied.
        vec![Finding::new(
            "retention.check",
            Severity::Info,
            format!("Retention check for '{name}' not yet implemented"),
        )]
    }

    fn check_space(&self, name: &str) -> Vec<Finding> {
        // TODO: check filesystem free space.
        vec![Finding::new(
            "space.check",
            Severity::Info,
            format!("Space check for '{name}' not yet implemented"),
        )]
    }
}

impl Default for Doctor {
    fn default() -> Self {
        Self::new()
    }
}
