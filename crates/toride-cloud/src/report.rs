//! Structured report types for cloud provider operations.
//!
//! Every mutating or diagnostic workflow in the crate returns one of these
//! report types so that callers can inspect results programmatically and
//! produce human-readable output independently.

// ---------------------------------------------------------------------------
// CloudReport
// ---------------------------------------------------------------------------

/// Aggregated report from a cloud provider operation.
///
/// Contains the detected provider, resolved security groups, and any
/// findings generated during the operation.
#[derive(Debug, Clone)]
pub struct CloudReport {
    /// The detected cloud provider.
    pub provider: crate::CloudProvider,
    /// Security groups discovered or managed.
    pub security_groups: Vec<crate::spec::SecurityGroup>,
    /// Findings produced during the operation.
    pub findings: Vec<Finding>,
}

impl CloudReport {
    /// Create an empty report for the given provider.
    #[must_use]
    pub fn new(provider: crate::CloudProvider) -> Self {
        Self {
            provider,
            security_groups: Vec::new(),
            findings: Vec::new(),
        }
    }

    /// Returns `true` if this report contains no security groups and no findings.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.security_groups.is_empty() && self.findings.is_empty()
    }

    /// Returns `true` if any finding has severity [`Severity::Error`] or higher.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|f| f.severity >= Severity::Error)
    }

    /// Add a finding to the report.
    pub fn push(&mut self, finding: Finding) {
        self.findings.push(finding);
    }
}

// ---------------------------------------------------------------------------
// Severity
// ---------------------------------------------------------------------------

/// Diagnostic severity level for findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

/// A single structured finding produced by a cloud operation or diagnostic.
///
/// Use [`Finding::new`] to construct the mandatory fields, then chain the
/// `.detail()` and `.fix()` builder methods for optional context.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Machine-readable dot-separated identifier,
    /// e.g. `"aws.security-group.open-ingress"`.
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
    #[must_use]
    pub fn new(id: impl Into<String>, severity: Severity, title: impl Into<String>) -> Self {
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
