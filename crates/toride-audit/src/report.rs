//! Audit report types for doctor findings and status summaries.
//!
//! Every diagnostic or mutating workflow in the crate returns one of these
//! report types so that callers can inspect results programmatically and
//! produce human-readable output independently.

// ---------------------------------------------------------------------------
// AuditReport
// ---------------------------------------------------------------------------

/// Aggregated audit report containing findings from a diagnostic run.
///
/// Provides convenience methods for summarising findings and checking
/// for blocking issues.
#[derive(Debug, Clone, Default)]
pub struct AuditReport {
    /// All findings collected during the audit run.
    pub findings: Vec<AuditFinding>,
    /// Summary statistics.
    pub summary: AuditSummary,
}

impl AuditReport {
    /// Create an empty report.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Add a finding to the report.
    pub fn push(&mut self, finding: AuditFinding) {
        self.findings.push(finding);
    }

    /// Returns the number of findings in this report.
    #[must_use]
    pub fn len(&self) -> usize {
        self.findings.len()
    }

    /// Returns `true` if this report contains no findings.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.findings.is_empty()
    }

    /// Returns `true` if any finding has severity [`AuditSeverity::Error`] or higher.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.findings
            .iter()
            .any(|f| f.severity >= AuditSeverity::Error)
    }
}

// ---------------------------------------------------------------------------
// AuditFinding
// ---------------------------------------------------------------------------

/// A single structured finding produced during an audit check.
///
/// Use [`AuditFinding::new`] to construct the mandatory fields, then chain
/// the `.detail()` and `.fix()` builder methods for optional context.
#[derive(Debug, Clone)]
pub struct AuditFinding {
    /// Machine-readable dot-separated identifier,
    /// e.g. `"binary.auditctl.missing"`.
    pub id: String,
    /// How severe this finding is.
    pub severity: AuditSeverity,
    /// Short human-readable title (one line).
    pub title: String,
    /// Longer description of the finding.
    pub detail: String,
    /// Suggested remediation action, if applicable.
    pub fix: Option<String>,
}

impl AuditFinding {
    /// Create a new finding with the mandatory fields.
    ///
    /// The `detail` and `fix` fields default to empty / `None` and can be
    /// filled in via the `.detail()` and `.fix()` chain methods.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        severity: AuditSeverity,
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

    /// Attach a longer description, replacing any previous detail text.
    #[must_use]
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }

    /// Attach a suggested fix action, replacing any previous fix.
    #[must_use]
    pub fn fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(fix.into());
        self
    }
}

// ---------------------------------------------------------------------------
// AuditSeverity
// ---------------------------------------------------------------------------

/// Diagnostic severity level for audit findings.
///
/// Ordered from least severe ([`AuditSeverity::Ok`]) to most severe
/// ([`AuditSeverity::Critical`]) so that reports can be sorted and filtered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuditSeverity {
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

impl std::fmt::Display for AuditSeverity {
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
// AuditSummary
// ---------------------------------------------------------------------------

/// Summary statistics for an audit report.
#[derive(Debug, Clone, Default)]
pub struct AuditSummary {
    /// Number of audit rules loaded.
    pub rules_loaded: usize,
    /// Whether auditd is running.
    pub auditd_running: bool,
    /// Whether AIDE database is initialized.
    pub aide_initialized: bool,
    /// Last AIDE check timestamp, if available.
    pub last_aide_check: Option<String>,
    /// Number of log files managed.
    pub log_files_count: usize,
}
