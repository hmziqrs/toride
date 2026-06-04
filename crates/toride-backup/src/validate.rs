//! Validation functions for backup specs, repository paths, retention
//! policies, and schedules.
//!
//! Ensures configuration values are well-formed and internally consistent,
//! returning structured [`ValidationFinding`]s that can be presented to the
//! user or logged.

use std::path::Path;

use crate::error::{Error, Result};
use crate::spec::{BackupSpec, Encryption, RetentionPolicy, Schedule};

/// Severity of a validation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationSeverity {
    /// Informational -- no action needed.
    Info,
    /// Warning -- may indicate a misconfiguration.
    Warning,
    /// Error -- invalid configuration.
    Error,
}

/// A single validation finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationFinding {
    /// The field or area this finding relates to.
    pub field: String,
    /// Severity of the finding.
    pub severity: ValidationSeverity,
    /// Human-readable description of the issue.
    pub message: String,
}

// ---------------------------------------------------------------------------
// Individual validators
// ---------------------------------------------------------------------------

/// Validate that a repository path is non-empty and well-formed.
///
/// The path must be non-empty and, if local, should look like an absolute path
/// or a known remote scheme prefix (`sftp:`, `s3:`, `b2:`, `rclone:`, `rest:`).
pub fn validate_repo_path(path: &Path) -> Result<()> {
    let s = path.as_os_str().to_string_lossy();

    if s.is_empty() {
        return Err(Error::ConfigParse(
            "repository path must not be empty".into(),
        ));
    }

    // Remote schemes are accepted as-is.
    let remote_schemes = ["sftp:", "s3:", "b2:", "rclone:", "rest:", "gs:", "azure:"];
    if remote_schemes.iter().any(|scheme| s.starts_with(scheme)) {
        return Ok(());
    }

    // Local paths should be absolute.
    if !s.starts_with('/') {
        return Err(Error::ConfigParse(format!(
            "local repository path should be absolute: {s}"
        )));
    }

    Ok(())
}

/// Validate a retention policy, returning a list of findings.
///
/// Checks:
/// - At least one `keep_*` value is set.
/// - Individual counts are reasonable (> 0, not excessively large).
pub fn validate_retention_policy(policy: &RetentionPolicy) -> Vec<ValidationFinding> {
    let mut findings = Vec::new();

    if !policy.has_any() {
        findings.push(ValidationFinding {
            field: "retention".into(),
            severity: ValidationSeverity::Error,
            message: "retention policy must have at least one keep-* value".into(),
        });
        return findings;
    }

    let counts = [
        ("keep_hourly", policy.keep_hourly),
        ("keep_daily", policy.keep_daily),
        ("keep_weekly", policy.keep_weekly),
        ("keep_monthly", policy.keep_monthly),
        ("keep_yearly", policy.keep_yearly),
    ];

    for (name, value) in counts {
        if let Some(n) = value {
            if n == 0 {
                findings.push(ValidationFinding {
                    field: format!("retention.{name}"),
                    severity: ValidationSeverity::Error,
                    message: format!("{name} is set to 0, which retains nothing"),
                });
            } else if n > 365 {
                findings.push(ValidationFinding {
                    field: format!("retention.{name}"),
                    severity: ValidationSeverity::Warning,
                    message: format!("{name} = {n} is very high; this may consume significant storage"),
                });
            }
        }
    }

    // Info: if only keep_yearly is set, snapshots may be far apart.
    if policy.keep_yearly.is_some()
        && policy.keep_hourly.is_none()
        && policy.keep_daily.is_none()
        && policy.keep_weekly.is_none()
        && policy.keep_monthly.is_none()
    {
        findings.push(ValidationFinding {
            field: "retention".into(),
            severity: ValidationSeverity::Info,
            message: "only keep_yearly is set; snapshots may be very far apart".into(),
        });
    }

    findings
}

/// Validate a schedule, returning a list of findings.
///
/// Checks:
/// - Cron expression has exactly 5 fields.
/// - Field values are within acceptable ranges.
pub fn validate_schedule(schedule: &Schedule) -> Vec<ValidationFinding> {
    let mut findings = Vec::new();
    let parts: Vec<&str> = schedule.cron.split_whitespace().collect();

    if parts.len() != 5 {
        findings.push(ValidationFinding {
            field: "schedule.cron".into(),
            severity: ValidationSeverity::Error,
            message: format!(
                "cron expression must have exactly 5 fields, got {}: {:?}",
                parts.len(),
                schedule.cron,
            ),
        });
        return findings;
    }

    // Basic range validation for simple numeric fields (does not handle
    // step values, ranges, or star syntax exhaustively -- just catches
    // obviously wrong values).
    let field_names = ["minute", "hour", "day-of-month", "month", "day-of-week"];
    let max_values: [u32; 5] = [59, 23, 31, 12, 7];

    for (i, (part, (name, &max))) in parts.iter().zip(field_names.iter().zip(&max_values)).enumerate()
    {
        // Skip wildcards, ranges, steps, and lists.
        if *part == "*" || part.contains('-') || part.contains('/') || part.contains(',') {
            continue;
        }

        if let Ok(val) = part.parse::<u32>() {
            let effective_min = if i == 2 { 1 } else { 0 }; // day-of-month starts at 1
            if val < effective_min || val > max {
                findings.push(ValidationFinding {
                    field: "schedule.cron".into(),
                    severity: ValidationSeverity::Error,
                    message: format!(
                        "cron {name} value {val} is out of range [{effective_min}, {max}]"
                    ),
                });
            }
        }
    }

    findings
}

// ---------------------------------------------------------------------------
// Full-spec validator
// ---------------------------------------------------------------------------

/// Validate a complete backup spec and return any findings.
///
/// Runs all sub-validators and additional cross-field checks:
/// - Name is non-empty.
/// - Repository path is valid.
/// - Sources are non-empty and absolute.
/// - Schedule is well-formed.
/// - Retention policy is reasonable.
/// - Encryption without `password_command` is warned about.
pub fn validate_spec(spec: &BackupSpec) -> Result<Vec<ValidationFinding>> {
    let mut findings = Vec::new();

    // --- Name ---
    if spec.name.trim().is_empty() {
        findings.push(ValidationFinding {
            field: "name".into(),
            severity: ValidationSeverity::Error,
            message: "backup spec name must not be empty".into(),
        });
    }

    // --- Repository path ---
    if let Err(e) = validate_repo_path(&spec.repository) {
        findings.push(ValidationFinding {
            field: "repository".into(),
            severity: ValidationSeverity::Error,
            message: e.to_string(),
        });
    }

    // --- Sources ---
    if spec.sources.is_empty() {
        findings.push(ValidationFinding {
            field: "sources".into(),
            severity: ValidationSeverity::Error,
            message: format!("backup spec {:?}: sources must not be empty", spec.name),
        });
    } else {
        for source in &spec.sources {
            let s = source.as_os_str().to_string_lossy();
            if s.is_empty() {
                findings.push(ValidationFinding {
                    field: "sources".into(),
                    severity: ValidationSeverity::Error,
                    message: format!(
                        "backup spec {:?}: source path must not be empty",
                        spec.name,
                    ),
                });
            } else if !s.starts_with('/') {
                findings.push(ValidationFinding {
                    field: "sources".into(),
                    severity: ValidationSeverity::Warning,
                    message: format!(
                        "backup spec {:?}: source path {:?} is not absolute",
                        spec.name, source,
                    ),
                });
            }
        }
    }

    // --- Schedule ---
    findings.extend(validate_schedule(&spec.schedule));

    // --- Retention ---
    findings.extend(validate_retention_policy(&spec.retention));

    // --- Encryption / password_command cross-check ---
    if spec.encryption != Encryption::None && spec.password_command.is_none() {
        findings.push(ValidationFinding {
            field: "password_command".into(),
            severity: ValidationSeverity::Warning,
            message: format!(
                "backup spec {:?}: encryption is {:?} but no password_command is set",
                spec.name, spec.encryption,
            ),
        });
    }

    // --- Encryption::None warning ---
    if spec.encryption == Encryption::None {
        findings.push(ValidationFinding {
            field: "encryption".into(),
            severity: ValidationSeverity::Warning,
            message: format!(
                "backup spec {:?}: encryption is set to 'none'; backups will be unencrypted",
                spec.name,
            ),
        });
    }

    // --- Duplicate tags ---
    let mut seen_tags = std::collections::HashSet::new();
    for tag in &spec.tags {
        if !seen_tags.insert(tag) {
            findings.push(ValidationFinding {
                field: "tags".into(),
                severity: ValidationSeverity::Warning,
                message: format!("duplicate tag: {tag}"),
            });
        }
    }

    Ok(findings)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Backend;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn sample_spec() -> BackupSpec {
        BackupSpec {
            name: "test-backup".into(),
            backend: Backend::Restic,
            repository: "/mnt/backups/test".into(),
            sources: vec![PathBuf::from("/etc"), PathBuf::from("/home")],
            schedule: Schedule::new("0 2 * * *"),
            retention: RetentionPolicy::default_policy(),
            encryption: Encryption::RepoKey,
            password_command: Some("cat /etc/restic/password".into()),
            exclude_patterns: vec![],
            tags: vec!["auto".into()],
            extra_env: HashMap::new(),
        }
    }

    #[test]
    fn valid_spec_has_no_errors() {
        let spec = sample_spec();
        let findings = validate_spec(&spec).unwrap();
        let errors: Vec<_> = findings
            .iter()
            .filter(|f| f.severity == ValidationSeverity::Error)
            .collect();
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn empty_name_flagged() {
        let mut spec = sample_spec();
        spec.name = "  ".into();
        let findings = validate_spec(&spec).unwrap();
        assert!(findings
            .iter()
            .any(|f| f.field == "name" && f.severity == ValidationSeverity::Error));
    }

    #[test]
    fn empty_sources_flagged() {
        let mut spec = sample_spec();
        spec.sources = vec![];
        let findings = validate_spec(&spec).unwrap();
        assert!(findings
            .iter()
            .any(|f| f.field == "sources" && f.severity == ValidationSeverity::Error));
    }

    #[test]
    fn relative_source_path_warned() {
        let mut spec = sample_spec();
        spec.sources = vec![PathBuf::from("relative/path")];
        let findings = validate_spec(&spec).unwrap();
        assert!(findings
            .iter()
            .any(|f| f.field == "sources" && f.severity == ValidationSeverity::Warning));
    }

    #[test]
    fn remote_repo_path_accepted() {
        assert!(validate_repo_path(Path::new("sftp:user@host:/backups")).is_ok());
        assert!(validate_repo_path(Path::new("s3:bucket/path")).is_ok());
    }

    #[test]
    fn local_relative_repo_rejected() {
        assert!(validate_repo_path(Path::new("relative/path")).is_err());
    }

    #[test]
    fn empty_repo_rejected() {
        assert!(validate_repo_path(Path::new("")).is_err());
    }

    #[test]
    fn bad_cron_fields() {
        let schedule = Schedule::new("0 2 * * * *");
        let findings = validate_schedule(&schedule);
        assert!(findings
            .iter()
            .any(|f| f.severity == ValidationSeverity::Error && f.field == "schedule.cron"));
    }

    #[test]
    fn good_cron_accepted() {
        let schedule = Schedule::new("0 2 * * *");
        let findings = validate_schedule(&schedule);
        assert!(findings.is_empty());
    }

    #[test]
    fn cron_out_of_range() {
        let schedule = Schedule::new("60 2 * * *");
        let findings = validate_schedule(&schedule);
        assert!(findings
            .iter()
            .any(|f| f.severity == ValidationSeverity::Error));
    }

    #[test]
    fn empty_retention_flagged() {
        let policy = RetentionPolicy {
            keep_hourly: None,
            keep_daily: None,
            keep_weekly: None,
            keep_monthly: None,
            keep_yearly: None,
        };
        let findings = validate_retention_policy(&policy);
        assert!(findings
            .iter()
            .any(|f| f.severity == ValidationSeverity::Error));
    }

    #[test]
    fn zero_retention_flagged() {
        let policy = RetentionPolicy {
            keep_daily: Some(0),
            ..RetentionPolicy::default()
        };
        let findings = validate_retention_policy(&policy);
        assert!(findings
            .iter()
            .any(|f| f.message.contains("set to 0") && f.severity == ValidationSeverity::Error));
    }

    #[test]
    fn encryption_without_password_warned() {
        let mut spec = sample_spec();
        spec.password_command = None;
        let findings = validate_spec(&spec).unwrap();
        assert!(findings
            .iter()
            .any(|f| f.field == "password_command" && f.severity == ValidationSeverity::Warning));
    }

    #[test]
    fn no_encryption_warned() {
        let mut spec = sample_spec();
        spec.encryption = Encryption::None;
        spec.password_command = None;
        let findings = validate_spec(&spec).unwrap();
        assert!(findings
            .iter()
            .any(|f| f.field == "encryption" && f.severity == ValidationSeverity::Warning));
    }

    #[test]
    fn duplicate_tags_flagged() {
        let mut spec = sample_spec();
        spec.tags = vec!["auto".into(), "auto".into()];
        let findings = validate_spec(&spec).unwrap();
        assert!(findings
            .iter()
            .any(|f| f.field == "tags" && f.message.contains("duplicate")));
    }

    #[test]
    fn only_yearly_retention_info() {
        let policy = RetentionPolicy {
            keep_hourly: None,
            keep_daily: None,
            keep_weekly: None,
            keep_monthly: None,
            keep_yearly: Some(3),
        };
        let findings = validate_retention_policy(&policy);
        assert!(findings
            .iter()
            .any(|f| f.severity == ValidationSeverity::Info));
    }
}
