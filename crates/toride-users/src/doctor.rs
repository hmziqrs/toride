//! Diagnostic checks for user and access control security.
//!
//! The doctor module runs a series of security checks and produces a
//! [`UserReport`] with findings. Checks include:
//!
//! - Root login enabled via SSH
//! - Users with empty passwords
//! - NOPASSWD sudo entries
//! - TOTP not configured for sudo users
//! - Insecure shells
//! - Password policy violations

use crate::paths::UserPaths;
use crate::report::{Severity, UserFinding, UserReport};
use crate::Result;

/// Scope for doctor checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorScope {
    /// Run all checks.
    All,
    /// Only check user account security.
    Accounts,
    /// Only check sudo configuration.
    Sudo,
    /// Only check PAM/TOTP configuration.
    Pam,
    /// Only check password policies.
    PasswordPolicy,
}

/// Diagnostic engine for user security checks.
pub struct Doctor {
    paths: UserPaths,
}

impl Doctor {
    /// Create a new doctor with the default system paths.
    #[must_use]
    pub fn new() -> Self {
        Self {
            paths: UserPaths::new(),
        }
    }

    /// Create a new doctor with custom paths.
    #[must_use]
    pub fn with_paths(paths: UserPaths) -> Self {
        Self { paths }
    }

    /// Run all checks in the given scope and return a report.
    ///
    /// # Errors
///
/// Returns an error only for fundamental failures (e.g. unreadable files).
/// Individual check failures appear as findings in the report.
    pub fn run(&self, scope: &DoctorScope) -> Result<UserReport> {
        let mut report = UserReport::new();

        match scope {
            DoctorScope::All => {
                self.check_accounts(&mut report)?;
                self.check_sudo(&mut report)?;
                self.check_pam(&mut report)?;
                self.check_password_policy(&mut report)?;
            }
            DoctorScope::Accounts => {
                self.check_accounts(&mut report)?;
            }
            DoctorScope::Sudo => {
                self.check_sudo(&mut report)?;
            }
            DoctorScope::Pam => {
                self.check_pam(&mut report)?;
            }
            DoctorScope::PasswordPolicy => {
                self.check_password_policy(&mut report)?;
            }
        }

        Ok(report)
    }

    /// Check user account security.
    fn check_accounts(&self, report: &mut UserReport) -> Result<()> {
        // Check for root login via SSH
        let sshd_config = std::path::Path::new("/etc/ssh/sshd_config");
        if sshd_config.exists() {
            let content = std::fs::read_to_string(sshd_config)?;
            if content.contains("PermitRootLogin yes") || content.contains("PermitRootLogin without-password") {
                // Only flag if it's explicitly "yes" (prohibit-password is often acceptable)
                if content.contains("PermitRootLogin yes") {
                    report.push(
                        UserFinding::new(
                            "user.root-login.ssh-enabled",
                            Severity::Critical,
                            "Root SSH login is enabled",
                        )
                        .detail("PermitRootLogin is set to 'yes' in /etc/ssh/sshd_config.")
                        .fix("Set PermitRootLogin to 'prohibit-password' or 'no'."),
                    );
                }
            }
        }

        // Check for users with UID 0 (root-equivalent)
        let passwd_entries = crate::parse::read_passwd(&self.paths.passwd)?;
        for entry in &passwd_entries {
            if entry.uid == 0 && entry.username != "root" {
                report.push(
                    UserFinding::new(
                        "user.uid-zero.non-root",
                        Severity::Critical,
                        format!("Non-root user '{}' has UID 0", entry.username),
                    )
                    .detail(format!(
                        "User '{}' has UID 0, granting full root privileges.",
                        entry.username
                    ))
                    .fix("Change the UID to a non-zero value or remove the user."),
                );
            }
        }

        // Check for users with login shells that shouldn't
        let insecure_shells = ["/bin/sh", "/bin/bash", "/usr/bin/bash"];
        let system_users = [
            "daemon", "bin", "sys", "sync", "games", "man", "lp", "mail",
            "news", "uucp", "proxy", "www-data", "backup", "list", "irc",
            "gnats", "nobody",
        ];
        for entry in &passwd_entries {
            if system_users.contains(&entry.username.as_str()) {
                if insecure_shells.contains(&entry.shell.as_str()) {
                    report.push(
                        UserFinding::new(
                            format!("user.system-user.shell.{}", entry.username),
                            Severity::Warning,
                            format!("System user '{}' has a login shell", entry.username),
                        )
                        .detail(format!(
                            "System user '{}' has shell '{}' instead of nologin.",
                            entry.username, entry.shell
                        ))
                        .fix("Set the shell to /usr/sbin/nologin."),
                    );
                }
            }
        }

        Ok(())
    }

    /// Check sudo configuration.
    fn check_sudo(&self, report: &mut UserReport) -> Result<()> {
        // Check main sudoers file for NOPASSWD entries
        if self.paths.sudoers.exists() {
            let entries = crate::parse::read_sudoers(&self.paths.sudoers)?;
            for entry in &entries {
                if entry.nopasswd {
                    report.push(
                        UserFinding::new(
                            "sudo.nopasswd.main-sudoers",
                            Severity::Warning,
                            format!("NOPASSWD sudo entry for '{}'", entry.who),
                        )
                        .detail(format!(
                            "User/group '{}' has NOPASSWD sudo access in main sudoers file.",
                            entry.who
                        ))
                        .fix("Remove NOPASSWD or require password authentication."),
                    );
                }
            }
        }

        // Check sudoers.d drop-in files
        if self.paths.sudoers_d.is_dir() {
            let entries = std::fs::read_dir(&self.paths.sudoers_d)?;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_none() || path.extension().is_some_and(|e| e != "bak") {
                    if let Ok(sudoers) = crate::parse::read_sudoers(&path) {
                        for rule in &sudoers {
                            if rule.nopasswd {
                                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                                report.push(
                                    UserFinding::new(
                                        format!("sudo.nopasswd.dropin.{filename}"),
                                        Severity::Warning,
                                        format!("NOPASSWD sudo entry in /etc/sudoers.d/{filename}"),
                                    )
                                    .detail(format!(
                                        "User/group '{}' has NOPASSWD access via drop-in file.",
                                        rule.who
                                    ))
                                    .fix("Remove NOPASSWD or require password + TOTP."),
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check PAM/TOTP configuration.
    fn check_pam(&self, report: &mut UserReport) -> Result<()> {
        // Check if TOTP is configured for SSH
        let sshd_pam = self.paths.pam_service("sshd");
        if sshd_pam.exists() {
            let rules = crate::pam::read_pam_config(&sshd_pam)?;
            let has_totp = rules
                .iter()
                .any(|r| r.module.contains("pam_google_authenticator"));

            if !has_totp {
                report.push(
                    UserFinding::new(
                        "pam.sshd.no-totp",
                        Severity::Warning,
                        "TOTP/2FA not configured for SSH",
                    )
                    .detail(
                        "The PAM configuration for sshd does not include \
                         pam_google_authenticator.so.",
                    )
                    .fix("Install libpam-google-authenticator and enable TOTP for SSH."),
                );
            }
        }

        // Check for sudo users without TOTP
        let _passwd_entries = crate::parse::read_passwd(&self.paths.passwd)?;
        let sudo_group_members = crate::parse::read_group(&self.paths.group)?
            .iter()
            .find(|g| g.name == "sudo")
            .map(|g| g.members.clone())
            .unwrap_or_default();

        for username in &sudo_group_members {
            if !crate::totp::is_totp_configured(username)? {
                report.push(
                    UserFinding::new(
                        format!("pam.sudo-user.no-totp.{username}"),
                        Severity::Info,
                        format!("Sudo user '{username}' does not have TOTP configured"),
                    )
                    .detail(format!(
                        "User '{username}' has sudo access but no TOTP/2FA.",
                    ))
                    .fix("Enroll the user in TOTP using google-authenticator."),
                );
            }
        }

        Ok(())
    }

    /// Check password policy compliance.
    fn check_password_policy(&self, report: &mut UserReport) -> Result<()> {
        // Check for users with empty passwords
        if self.paths.shadow.exists() {
            let shadow = std::fs::read_to_string(&self.paths.shadow)?;
            for line in shadow.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 && !parts[0].starts_with('#') {
                    let username = parts[0];
                    // Empty password field
                    if parts[1].is_empty() {
                        report.push(
                            UserFinding::new(
                                format!("password.empty.{username}"),
                                Severity::Critical,
                                format!("User '{username}' has an empty password"),
                            )
                            .detail(format!(
                                "User '{username}' has no password set in /etc/shadow.",
                            ))
                            .fix("Set a strong password or lock the account."),
                        );
                    }
                }
            }
        }

        // Check login.defs for password policy
        if self.paths.login_defs.exists() {
            let content = std::fs::read_to_string(&self.paths.login_defs)?;
            let has_max_days = content.contains("PASS_MAX_DAYS");
            let has_min_days = content.contains("PASS_MIN_DAYS");

            if !has_max_days {
                report.push(
                    UserFinding::new(
                        "password-policy.no-max-days",
                        Severity::Warning,
                        "No PASS_MAX_DAYS set in /etc/login.defs",
                    )
                    .detail("Password expiration is not configured.")
                    .fix("Set PASS_MAX_DAYS to 90 or less in /etc/login.defs."),
                );
            }

            if !has_min_days {
                report.push(
                    UserFinding::new(
                        "password-policy.no-min-days",
                        Severity::Info,
                        "No PASS_MIN_DAYS set in /etc/login.defs",
                    )
                    .detail("Minimum password change interval is not configured.")
                    .fix("Set PASS_MIN_DAYS to at least 1 in /etc/login.defs."),
                );
            }
        }

        Ok(())
    }
}

impl Default for Doctor {
    fn default() -> Self {
        Self::new()
    }
}
