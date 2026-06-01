//! Validation functions for audit rules and configuration.
//!
//! Provides pre-flight validation for audit rules and AIDE configuration
//! strings before they are applied to the system.

// ---------------------------------------------------------------------------
// Audit rule validation
// ---------------------------------------------------------------------------

/// Validate a single audit rule string.
///
/// Checks that the rule starts with a recognized flag (`-a`, `-A`, `-d`,
/// `-D`, `-S`, `-F`, `-m`, `-s`, `-l`, `-e`, `-k`, `-w`, `-p`, `-W`).
///
/// # Errors
///
/// Returns [`crate::Error::AuditRuleParse`] if the rule is malformed.
pub fn validate_audit_rule(rule: &str) -> crate::Result<()> {
    let trimmed = rule.trim();

    // Skip empty lines and comments.
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(());
    }

    // Check for a recognized flag prefix.
    let valid_prefixes = [
        "-a ", "-A ", "-d ", "-D ", "-S ", "-F ", "-m ", "-s ", "-l ", "-e ",
        "-k ", "-w ", "-p ", "-W ",
    ];

    let has_valid_prefix = valid_prefixes
        .iter()
        .any(|prefix| trimmed.starts_with(prefix));

    if has_valid_prefix {
        Ok(())
    } else {
        Err(crate::Error::AuditRuleParse(format!(
            "unrecognized audit rule: {trimmed}"
        )))
    }
}

// ---------------------------------------------------------------------------
// AIDE configuration validation
// ---------------------------------------------------------------------------

/// Validate an AIDE configuration string.
///
/// Performs basic syntax checks: ensures database paths are specified
/// and monitored paths are absolute.
///
/// # Errors
///
/// Returns [`crate::Error::AideError`] if the configuration is invalid.
pub fn validate_aide_config(config: &str) -> crate::Result<()> {
    let has_database = config
        .lines()
        .any(|line| line.trim().starts_with("database="));

    if !has_database {
        return Err(crate::Error::AideError(
            "AIDE configuration must specify a database path".to_owned(),
        ));
    }

    // Check that path entries are absolute.
    for line in config.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.contains('=') {
            continue;
        }
        // Lines that look like path entries should start with '/'.
        if !trimmed.starts_with('/') && !trimmed.starts_with('!') {
            return Err(crate::Error::AideError(format!(
                "AIDE path entry must be absolute: {trimmed}"
            )));
        }
    }

    Ok(())
}
