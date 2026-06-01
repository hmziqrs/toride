//! Path constants and resolution for audit subsystem configuration files.
//!
//! Provides well-known paths for the Linux audit daemon (`/etc/audit/`),
//! AIDE configuration (`/etc/aide.conf`), rsyslog (`/etc/rsyslog.conf`),
//! and logrotate drop-in directory (`/etc/logrotate.d/`).

use std::path::PathBuf;

/// Well-known system paths used by the audit subsystem.
///
/// These are the default Linux FHS locations. Production code should
/// prefer [`crate::AuditPaths`] which supports override, but these
/// constants are useful for validation and display.
pub struct AuditPathsConst;

impl AuditPathsConst {
    /// Default audit daemon configuration directory.
    pub const AUDIT_DIR: &'static str = "/etc/audit";

    /// Default audit rules directory.
    pub const RULES_D: &'static str = "/etc/audit/rules.d";

    /// Default auditd configuration file.
    pub const AUDITD_CONF: &'static str = "/etc/audit/auditd.conf";

    /// Default AIDE configuration file.
    pub const AIDE_CONF: &'static str = "/etc/aide.conf";

    /// Default rsyslog configuration file.
    pub const RSYSLOG_CONF: &'static str = "/etc/rsyslog.conf";

    /// Default rsyslog drop-in directory.
    pub const RSYSLOG_D: &'static str = "/etc/rsyslog.d";

    /// Default logrotate drop-in directory.
    pub const LOGROTATE_D: &'static str = "/etc/logrotate.d";

    /// Default AIDE database directory.
    pub const AIDE_DB_DIR: &'static str = "/var/lib/aide";
}

/// Resolves the path to a managed audit rules file.
///
/// Returns `{rules_d}/{name}.rules` using the default rules directory.
#[must_use]
pub fn rules_file(name: &str) -> PathBuf {
    PathBuf::from(AuditPathsConst::RULES_D).join(format!("{name}.rules"))
}

/// Resolves the path to a managed logrotate config file.
///
/// Returns `{logrotate_d}/{name}` using the default logrotate directory.
#[must_use]
pub fn logrotate_file(name: &str) -> PathBuf {
    PathBuf::from(AuditPathsConst::LOGROTATE_D).join(name)
}

/// Resolves the path to a managed rsyslog drop-in file.
///
/// Returns `{rsyslog_d}/{name}.conf` using the default rsyslog directory.
#[must_use]
pub fn rsyslog_dropin(name: &str) -> PathBuf {
    PathBuf::from(AuditPathsConst::RSYSLOG_D).join(format!("{name}.conf"))
}
