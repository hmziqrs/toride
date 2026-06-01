//! A single diagnostic finding.

use crate::Severity;

/// One discrete check result produced by a doctor run.
///
/// Use the builder-pattern methods to construct a finding fluently:
///
/// ```
/// use toride_diagnostic_types::{Finding, Severity};
///
/// let f = Finding::new("ssh:key-exists", Severity::Critical, "No SSH key found")
///     .domain("ssh")
///     .detail("Expected a key at ~/.ssh/id_ed25519")
///     .fix_hint("Run: ssh-keygen -t ed25519");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Finding {
    /// Unique identifier for this check (e.g. `"ssh:key-exists"`).
    pub id: String,
    /// How serious this finding is.
    pub severity: Severity,
    /// Short, human-readable summary.
    pub message: String,
    /// Optional longer explanation.
    pub detail: Option<String>,
    /// Optional hint describing how to fix the issue.
    pub fix_hint: Option<String>,
    /// Domain the finding belongs to (e.g. `"ssh"`, `"firewall"`).
    pub domain: String,
}

impl Finding {
    /// Create a new finding with the required fields.
    ///
    /// The `domain` defaults to `"general"`; override it with [`Self::domain`].
    #[must_use]
    pub fn new(id: impl Into<String>, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            severity,
            message: message.into(),
            detail: None,
            fix_hint: None,
            domain: String::from("general"),
        }
    }

    /// Set the domain.
    #[must_use]
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = domain.into();
        self
    }

    /// Attach a longer explanation.
    #[must_use]
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Attach a suggested fix.
    #[must_use]
    pub fn fix_hint(mut self, hint: impl Into<String>) -> Self {
        self.fix_hint = Some(hint.into());
        self
    }

    /// Shorthand for a critical-severity finding.
    #[must_use]
    pub fn critical(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, Severity::Critical, message)
    }

    /// Shorthand for an important-severity finding.
    #[must_use]
    pub fn important(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, Severity::Important, message)
    }

    /// Shorthand for a warning-severity finding.
    #[must_use]
    pub fn warning(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, Severity::Warning, message)
    }

    /// Shorthand for an info-severity finding.
    #[must_use]
    pub fn info(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, Severity::Info, message)
    }

    /// Shorthand for an ok-severity finding (check passed).
    #[must_use]
    pub fn ok(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, Severity::Ok, message)
    }

    /// Returns `true` if the finding severity is [`Severity::Ok`].
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.severity == Severity::Ok
    }

    /// Returns `true` if the finding severity is [`Severity::Critical`] or [`Severity::Important`].
    #[must_use]
    pub fn is_actionable(&self) -> bool {
        matches!(self.severity, Severity::Critical | Severity::Important)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_sets_fields() {
        let f = Finding::new("test:check", Severity::Warning, "something off")
            .domain("ssh")
            .detail("more info")
            .fix_hint("do the thing");

        assert_eq!(f.id, "test:check");
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.message, "something off");
        assert_eq!(f.detail.as_deref(), Some("more info"));
        assert_eq!(f.fix_hint.as_deref(), Some("do the thing"));
        assert_eq!(f.domain, "ssh");
    }

    #[test]
    fn is_ok_and_is_actionable() {
        let ok = Finding::new("a", Severity::Ok, "fine");
        let crit = Finding::new("b", Severity::Critical, "broken");
        let warn = Finding::new("c", Severity::Warning, "meh");

        assert!(ok.is_ok());
        assert!(!ok.is_actionable());
        assert!(!crit.is_ok());
        assert!(crit.is_actionable());
        assert!(!warn.is_actionable());
    }
}
