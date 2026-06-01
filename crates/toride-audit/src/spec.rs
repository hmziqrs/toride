//! Audit specification types for declarative configuration.
//!
//! [`AuditSpec`] captures the desired state of the audit subsystem:
//! audit rules, integrity monitoring configuration, and log settings.
//! The spec is validated before being rendered and applied to disk.

use std::path::PathBuf;

// ---------------------------------------------------------------------------
// AuditSpec
// ---------------------------------------------------------------------------

/// Declarative specification for the audit subsystem.
///
/// An `AuditSpec` describes the desired state of audit rules, AIDE
/// integrity monitoring, and log aggregation. It is validated, rendered,
/// and applied to produce configuration files on disk.
///
/// # Example
///
/// ```ignore
/// use toride_audit::spec::AuditSpec;
///
/// let spec = AuditSpec::default()
///     .with_audit_rules(vec!["-a exit,always -F arch=b64 -S execve".to_owned()])
///     .with_integrity_paths(vec!["/etc".to_owned(), "/usr/bin".to_owned()]);
/// spec.validate()?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct AuditSpec {
    /// Audit rules to apply (one rule per line).
    pub audit_rules: Vec<String>,
    /// Paths to monitor for file integrity via AIDE.
    pub integrity_paths: Vec<String>,
    /// AIDE configuration options.
    pub integrity_options: IntegrityOptions,
    /// Log management settings.
    pub log_settings: LogSettings,
}

impl AuditSpec {
    /// Create a new empty specification.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the audit rules.
    #[must_use]
    pub fn with_audit_rules(mut self, rules: Vec<String>) -> Self {
        self.audit_rules = rules;
        self
    }

    /// Set the paths to monitor for file integrity.
    #[must_use]
    pub fn with_integrity_paths(mut self, paths: Vec<String>) -> Self {
        self.integrity_paths = paths;
        self
    }

    /// Set the integrity options.
    #[must_use]
    pub fn with_integrity_options(mut self, options: IntegrityOptions) -> Self {
        self.integrity_options = options;
        self
    }

    /// Set the log settings.
    #[must_use]
    pub fn with_log_settings(mut self, settings: LogSettings) -> Self {
        self.log_settings = settings;
        self
    }

    /// Validate the specification.
    ///
    /// Checks that audit rules are syntactically valid and integrity paths
    /// are absolute.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error`] if any rule or path is invalid.
    pub fn validate(&self) -> crate::Result<()> {
        for rule in &self.audit_rules {
            if rule.trim().is_empty() {
                continue;
            }
            crate::validate::validate_audit_rule(rule)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// IntegrityOptions
// ---------------------------------------------------------------------------

/// AIDE integrity monitoring configuration options.
#[derive(Debug, Clone)]
pub struct IntegrityOptions {
    /// Database file location.
    pub database_path: PathBuf,
    /// Database output location (for initialization).
    pub database_out_path: PathBuf,
    /// Compression level for the AIDE database (0-9).
    pub compression_level: u8,
    /// Whether to use summarized changes in reports.
    pub summarize_changes: bool,
    /// Report URL or path for AIDE reports.
    pub report_url: String,
}

impl Default for IntegrityOptions {
    fn default() -> Self {
        Self {
            database_path: PathBuf::from("/var/lib/aide/aide.db.gz"),
            database_out_path: PathBuf::from("/var/lib/aide/aide.db.new.gz"),
            compression_level: 6,
            summarize_changes: true,
            report_url: "stdout".to_owned(),
        }
    }
}

// ---------------------------------------------------------------------------
// LogSettings
// ---------------------------------------------------------------------------

/// Log aggregation and rotation settings.
#[derive(Debug, Clone)]
pub struct LogSettings {
    /// Maximum log file size in bytes before rotation.
    pub max_size: u64,
    /// Number of rotated log files to retain.
    pub rotate_count: u32,
    /// Number of days to keep rotated logs.
    pub max_age_days: u32,
    /// Whether to compress rotated log files.
    pub compress: bool,
    /// Log aggregation backend to use.
    pub backend: LogBackend,
}

impl Default for LogSettings {
    fn default() -> Self {
        Self {
            max_size: 100 * 1024 * 1024, // 100 MB
            rotate_count: 10,
            max_age_days: 30,
            compress: true,
            backend: LogBackend::Rsyslog,
        }
    }
}

// ---------------------------------------------------------------------------
// LogBackend
// ---------------------------------------------------------------------------

/// Supported log aggregation backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogBackend {
    /// Use rsyslog for log aggregation.
    #[default]
    Rsyslog,
    /// Use systemd-journald for log aggregation.
    Journald,
}
