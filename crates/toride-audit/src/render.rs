//! Configuration file renderers for audit subsystems.
//!
//! Converts structured types into text suitable for writing to
//! `/etc/audit/`, `/etc/aide.conf`, and `/etc/rsyslog.conf`.

// ---------------------------------------------------------------------------
// Audit rules renderer
// ---------------------------------------------------------------------------

/// Render a list of audit rules into a rules file suitable for
/// `/etc/audit/rules.d/`.
///
/// Each rule is emitted on its own line. Empty lines and comments
/// (lines starting with `#`) are preserved as-is.
pub fn render_audit_rules(rules: &[String]) -> String {
    rules
        .iter()
        .map(|r| r.as_str())
        .collect::<Vec<&str>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// AIDE configuration renderer
// ---------------------------------------------------------------------------

/// Render an AIDE configuration string from integrity options.
///
/// Produces a minimal but functional `aide.conf` with database paths,
/// custom groups, and monitored directories.
///
/// # Arguments
///
/// * `database_path` - Path to the AIDE reference database.
/// * `database_out_path` - Path for the new database after initialization.
/// * `monitored_paths` - Directories and files to monitor.
/// * `report_url` - Where to send reports (e.g. `stdout`, `file:/var/log/aide.report`).
pub fn render_aide_config(
    database_path: &str,
    database_out_path: &str,
    monitored_paths: &[String],
    report_url: &str,
) -> String {
    let mut out = String::new();

    out.push_str(&format!("database=file:{database_path}\n"));
    out.push_str(&format!("database_out=file:{database_out_path}\n"));
    out.push_str(&format!("report_url={report_url}\n"));
    out.push('\n');

    for path in monitored_paths {
        out.push_str(&format!("{path} ALL\n"));
    }

    out
}

// ---------------------------------------------------------------------------
// rsyslog configuration renderer
// ---------------------------------------------------------------------------

/// Render an rsyslog configuration snippet for audit log forwarding.
///
/// Produces configuration lines suitable for a drop-in file in
/// `/etc/rsyslog.d/`.
///
/// # Arguments
///
/// * `facility` - The syslog facility (e.g. `authpriv`, `local6`).
/// * `log_path` - Path to the log file.
/// * `template` - Optional template name for log formatting.
pub fn render_rsyslog_config(
    facility: &str,
    log_path: &str,
    template: Option<&str>,
) -> String {
    let template_directive = template
        .map(|t| format!(";{t}"))
        .unwrap_or_default();
    format!("{facility}.*\t{log_path}{template_directive}\n")
}
