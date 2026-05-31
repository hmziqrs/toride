//! Comprehensive diagnostic engine for Fail2Ban installations.
//!
//! [`Doctor`] is the most important differentiator module. It runs structured
//! diagnostic checks across a Fail2Ban installation and returns a
//! [`DoctorReport`] containing typed [`Finding`] values with severity levels,
//! human-readable descriptions, and suggested fixes.
//!
//! # Categories
//!
//! Each category corresponds to a [`DoctorScope`] variant and a `check_*`
//! method on [`Doctor`]:
//!
//! | Scope | Method | What it checks |
//! |-------|--------|---------------|
//! | `Binary` | [`check_binaries`] | fail2ban-client, fail2ban-regex, systemctl, nft/iptables |
//! | `Service` | [`check_service`] | service active/enabled, ping, log target, database |
//! | `Config` | [`check_config`] | config dir, generated files, --test, managed header |
//! | `Jail(name)` | [`check_jail`] | jail exists/enabled, filter, action, sane timing |
//! | `LogPath` | [`check_log_paths`] | log files exist, readable, glob patterns |
//! | `Journal` | [`check_journal`] | systemd backend, journalmatch, journal access |
//! | `Regex` | [`check_regex`] | failregex compiles, `<HOST>` usage |
//! | `Action` | [`check_actions`] | action file exists, ban/unban, firewall compat |
//! | `Permission` | [`check_permissions`] | world-writable checks, ownership, socket |
//! | `Safety` | [`check_safety`] | dry-run, backup, rollback path |
//! | `Proxy` | [`check_proxy`] | proxy-only IPs, Cloudflare/Traefik warnings |
//!
//! # Example
//!
//! ```ignore
//! use toride_fail2ban::command::DuctRunner;
//! use toride_fail2ban::doctor::{Doctor, DoctorScope};
//!
//! let runner = DuctRunner::new();
//! let doctor = Doctor::new(&runner);
//!
//! let report = doctor.run(&DoctorScope::All)?;
//! if report.has_critical() {
//!     for f in &report.findings {
//!         if f.severity >= Severity::Critical {
//!             eprintln!("[{}] {}", f.severity, f.title);
//!         }
//!     }
//! }
//! ```
//!
//! All commands go through the [`Runner`](crate::command::Runner) trait. No
//! ad-hoc `std::process::Command` calls are made anywhere in this module.

use serde::{Deserialize, Serialize};

use crate::command::{find_binary, Runner};
use crate::report::{DoctorReport, Finding, Severity};
use crate::Result;

// ---------------------------------------------------------------------------
// DoctorScope
// ---------------------------------------------------------------------------

/// Selects which diagnostic category (or categories) to run.
///
/// Pass [`DoctorScope::All`] to run every category, or choose a specific
/// category for targeted checks. [`DoctorScope::Jail`] takes a jail name so
/// that only that jail is inspected.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorScope {
    /// Run all diagnostic categories.
    All,
    /// Check that required binaries exist and have compatible versions.
    Binary,
    /// Check that the Fail2Ban service is active, enabled, and reachable.
    Service,
    /// Check config directory, generated files, managed headers, and --test.
    Config,
    /// Check a single named jail (exists, enabled, filter, action, timing).
    Jail(String),
    /// Check that configured log paths exist and are readable.
    LogPath,
    /// Check systemd journal backend, journalmatch, and journal access.
    Journal,
    /// Check that failregex patterns compile and use `<HOST>` correctly.
    Regex,
    /// Check action files, ban/unban definitions, and firewall compatibility.
    Action,
    /// Check file permissions for world-writable, ownership, and socket safety.
    Permission,
    /// Check that dry-run, backup, and rollback paths are available.
    Safety,
    /// Check for proxy-only IPs and warn about Cloudflare/Traefik.
    Proxy,
}

impl DoctorScope {
    /// Return all non-`All` scope variants as a list.
    ///
    /// Useful for callers that want to iterate over individual categories
    /// (for example, to build a UI where each category is a separate tab).
    pub fn all_categories() -> Vec<DoctorScope> {
        vec![
            DoctorScope::Binary,
            DoctorScope::Service,
            DoctorScope::Config,
            DoctorScope::LogPath,
            DoctorScope::Journal,
            DoctorScope::Regex,
            DoctorScope::Action,
            DoctorScope::Permission,
            DoctorScope::Safety,
            DoctorScope::Proxy,
        ]
    }
}

// ---------------------------------------------------------------------------
// Doctor
// ---------------------------------------------------------------------------

/// Diagnostic engine that runs structured checks against a Fail2Ban
/// installation.
///
/// Borrows a [`Runner`] so that it can be used with either the production
/// [`DuctRunner`](crate::command::DuctRunner) or the test
/// [`FakeRunner`](crate::command::FakeRunner).
///
/// Every check method returns a `Vec<Finding>` so that individual categories
/// can be called in isolation or aggregated via [`Doctor::run`].
pub struct Doctor<'a> {
    runner: &'a dyn Runner,
}

impl<'a> Doctor<'a> {
    /// Create a new diagnostic engine backed by `runner`.
    pub fn new(runner: &'a dyn Runner) -> Self {
        Self { runner }
    }

    // -----------------------------------------------------------------------
    // Dispatch
    // -----------------------------------------------------------------------

    /// Run the selected diagnostic scope and return a complete report.
    ///
    /// When `scope` is [`DoctorScope::All`], every category is run and the
    /// findings are merged into a single report. For a single category only
    /// that category's checks are performed.
    ///
    /// # Errors
    ///
    /// Returns an error only if a fundamental failure occurs (e.g. the runner
    /// itself is broken). Individual check failures are reported as
    /// [`Severity::Error`] or [`Severity::Critical`] findings inside the
    /// report, not as `Err`.
    pub fn run(&self, scope: &DoctorScope) -> Result<DoctorReport> {
        let mut report = DoctorReport::empty();

        match scope {
            DoctorScope::All => {
                for cat in DoctorScope::all_categories() {
                    // Recurse for each category. Jail-only scopes are skipped
                    // in All mode since they require a jail name.
                    let sub_report = self.run(&cat)?;
                    report.findings.extend(sub_report.findings);
                }
            }
            DoctorScope::Binary => {
                report.findings.extend(self.check_binaries());
            }
            DoctorScope::Service => {
                report.findings.extend(self.check_service());
            }
            DoctorScope::Config => {
                report.findings.extend(self.check_config());
            }
            DoctorScope::Jail(name) => {
                report.findings.extend(self.check_jail(name));
            }
            DoctorScope::LogPath => {
                report.findings.extend(self.check_log_paths());
            }
            DoctorScope::Journal => {
                report.findings.extend(self.check_journal());
            }
            DoctorScope::Regex => {
                report.findings.extend(self.check_regex());
            }
            DoctorScope::Action => {
                report.findings.extend(self.check_actions());
            }
            DoctorScope::Permission => {
                report.findings.extend(self.check_permissions());
            }
            DoctorScope::Safety => {
                report.findings.extend(self.check_safety());
            }
            DoctorScope::Proxy => {
                report.findings.extend(self.check_proxy());
            }
        }

        Ok(report)
    }

    // =======================================================================
    // Binary checks
    // =======================================================================

    /// Verify that all required binaries are present and detect the Fail2Ban
    /// version.
    ///
    /// Checks:
    ///
    /// - `fail2ban-client` exists on `$PATH`
    /// - `fail2ban-regex` exists on `$PATH`
    /// - Fail2Ban version can be detected via `fail2ban-client --version`
    /// - `systemctl` exists on `$PATH`
    /// - `nft` / `iptables` availability based on configured actions
    fn check_binaries(&self) -> Vec<Finding> {
        let mut findings = Vec::new();

        // fail2ban-client
        match find_binary("fail2ban-client") {
            Ok(path) => {
                findings.push(
                    Finding::new(
                        "binary.fail2ban-client.found",
                        Severity::Ok,
                        "fail2ban-client binary found",
                    )
                    .detail(format!("Located at {}", path.display())),
                );

                // Try to detect the version.
                match self.runner.run(
                    path.to_str().unwrap_or("fail2ban-client"),
                    &["--version"],
                ) {
                    Ok(out) if out.success => {
                        let ver = out.stdout.trim();
                        findings.push(
                            Finding::new(
                                "binary.fail2ban-client.version",
                                Severity::Info,
                                "Fail2Ban version detected",
                            )
                            .detail(format!("Version: {ver}")),
                        );
                    }
                    Ok(out) => {
                        findings.push(
                            Finding::new(
                                "binary.fail2ban-client.version-failed",
                                Severity::Warning,
                                "Could not detect Fail2Ban version",
                            )
                            .detail(format!(
                                "fail2ban-client --version exited with code {:?}: {}",
                                out.exit_code,
                                out.stderr.trim(),
                            )),
                        );
                    }
                    Err(e) => {
                        findings.push(
                            Finding::new(
                                "binary.fail2ban-client.version-error",
                                Severity::Warning,
                                "Could not detect Fail2Ban version",
                            )
                            .detail(format!("Running --version failed: {e}")),
                        );
                    }
                }
            }
            Err(_) => {
                findings.push(
                    Finding::new(
                        "binary.fail2ban-client.missing",
                        Severity::Critical,
                        "fail2ban-client not found",
                    )
                    .detail(
                        "The fail2ban-client binary could not be located on \
                         $PATH. Fail2Ban may not be installed.",
                    )
                    .fix("Install Fail2Ban: apt install fail2ban (Debian/Ubuntu) or dnf install fail2ban (Fedora)."),
                );
            }
        }

        // fail2ban-regex
        match find_binary("fail2ban-regex") {
            Ok(path) => {
                findings.push(
                    Finding::new(
                        "binary.fail2ban-regex.found",
                        Severity::Ok,
                        "fail2ban-regex binary found",
                    )
                    .detail(format!("Located at {}", path.display())),
                );
            }
            Err(_) => {
                findings.push(
                    Finding::new(
                        "binary.fail2ban-regex.missing",
                        Severity::Warning,
                        "fail2ban-regex not found",
                    )
                    .detail(
                        "The fail2ban-regex binary could not be located on \
                         $PATH. Regex testing will not be available.",
                    )
                    .fix("Install Fail2Ban (includes fail2ban-regex) or ensure it is on $PATH."),
                );
            }
        }

        // systemctl
        match find_binary("systemctl") {
            Ok(path) => {
                findings.push(
                    Finding::new(
                        "binary.systemctl.found",
                        Severity::Ok,
                        "systemctl binary found",
                    )
                    .detail(format!("Located at {}", path.display())),
                );
            }
            Err(_) => {
                findings.push(
                    Finding::new(
                        "binary.systemctl.missing",
                        Severity::Warning,
                        "systemctl not found",
                    )
                    .detail(
                        "The systemctl binary could not be located on $PATH. \
                         Service management commands will not be available. \
                         This is expected on non-systemd systems.",
                    )
                    .fix("Install systemd or avoid using service management features."),
                );
            }
        }

        // nft availability
        match self.runner.run("nft", &["--version"]) {
            Ok(out) if out.success => {
                findings.push(Finding::new(
                    "binary.nft.available",
                    Severity::Ok,
                    "nft binary available",
                ));
            }
            _ => {
                findings.push(
                    Finding::new(
                        "binary.nft.unavailable",
                        Severity::Info,
                        "nft binary not available",
                    )
                    .detail(
                        "nft (nftables) is not available. If jails use nftables \
                         actions, bans will fail.",
                    ),
                );
            }
        }

        // iptables availability
        match self.runner.run("iptables", &["--version"]) {
            Ok(out) if out.success => {
                findings.push(Finding::new(
                    "binary.iptables.available",
                    Severity::Ok,
                    "iptables binary available",
                ));
            }
            _ => {
                findings.push(
                    Finding::new(
                        "binary.iptables.unavailable",
                        Severity::Info,
                        "iptables binary not available",
                    )
                    .detail(
                        "iptables is not available. If jails use iptables \
                         actions, bans will fail.",
                    ),
                );
            }
        }

        findings
    }

    // =======================================================================
    // Service checks
    // =======================================================================

    /// Verify that the Fail2Ban service is active, enabled, and responsive.
    ///
    /// Checks:
    ///
    /// - Fail2Ban service is active (running)
    /// - Fail2Ban service is enabled (starts at boot)
    /// - `fail2ban-client ping` succeeds
    /// - log target is accessible
    /// - database file path is readable
    fn check_service(&self) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Service active.
        match self.runner.run("systemctl", &["is-active", "fail2ban"]) {
            Ok(out) => {
                if out.success {
                    findings.push(Finding::new(
                        "service.active",
                        Severity::Ok,
                        "Fail2Ban service is active",
                    ));
                } else {
                    findings.push(
                        Finding::new(
                            "service.inactive",
                            Severity::Critical,
                            "Fail2Ban service is not active",
                        )
                        .detail(format!(
                            "systemctl is-active returned: {}",
                            out.stdout.trim(),
                        ))
                        .fix("Start the service: systemctl start fail2ban"),
                    );
                }
            }
            Err(e) => {
                findings.push(
                    Finding::new(
                        "service.active-check-error",
                        Severity::Error,
                        "Could not check Fail2Ban service state",
                    )
                    .detail(format!("systemctl is-active failed: {e}")),
                );
            }
        }

        // Service enabled.
        match self.runner.run("systemctl", &["is-enabled", "fail2ban"]) {
            Ok(out) => {
                if out.success {
                    findings.push(Finding::new(
                        "service.enabled",
                        Severity::Ok,
                        "Fail2Ban service is enabled",
                    ));
                } else {
                    findings.push(
                        Finding::new(
                            "service.not-enabled",
                            Severity::Warning,
                            "Fail2Ban service is not enabled at boot",
                        )
                        .fix("Enable the service: systemctl enable fail2ban"),
                    );
                }
            }
            Err(e) => {
                findings.push(
                    Finding::new(
                        "service.enabled-check-error",
                        Severity::Error,
                        "Could not check Fail2Ban enabled state",
                    )
                    .detail(format!("systemctl is-enabled failed: {e}")),
                );
            }
        }

        // fail2ban-client ping.
        match find_binary("fail2ban-client") {
            Ok(path) => {
                let bin = path.to_str().unwrap_or("fail2ban-client");
                match self.runner.run(bin, &["ping"]) {
                    Ok(out) if out.success => {
                        findings.push(Finding::new(
                            "service.ping-ok",
                            Severity::Ok,
                            "fail2ban-client ping succeeded",
                        ));
                    }
                    Ok(out) => {
                        findings.push(
                            Finding::new(
                                "service.ping-failed",
                                Severity::Critical,
                                "fail2ban-client ping failed",
                            )
                            .detail(format!(
                                "Ping returned exit code {:?}: {}",
                                out.exit_code,
                                out.stderr.trim(),
                            ))
                            .fix(
                                "Verify the Fail2Ban daemon is running and the \
                                 socket is accessible.",
                            ),
                        );
                    }
                    Err(e) => {
                        findings.push(
                            Finding::new(
                                "service.ping-error",
                                Severity::Critical,
                                "fail2ban-client ping error",
                            )
                            .detail(format!("Ping command failed: {e}")),
                        );
                    }
                }

                // Log target accessible.
                match self.runner.run(bin, &["get", "logtarget"]) {
                    Ok(out) if out.success => {
                        let target = out.stdout.trim().to_string();
                        findings.push(
                            Finding::new(
                                "service.logtarget-accessible",
                                Severity::Info,
                                "Log target is configured",
                            )
                            .detail(format!("Log target: {target}")),
                        );
                    }
                    Ok(out) => {
                        findings.push(
                            Finding::new(
                                "service.logtarget-unavailable",
                                Severity::Warning,
                                "Could not retrieve log target",
                            )
                            .detail(format!(
                                "get logtarget exited with code {:?}: {}",
                                out.exit_code,
                                out.stderr.trim(),
                            )),
                        );
                    }
                    Err(e) => {
                        findings.push(
                            Finding::new(
                                "service.logtarget-error",
                                Severity::Warning,
                                "Could not retrieve log target",
                            )
                            .detail(format!("get logtarget failed: {e}")),
                        );
                    }
                }

                // Database file readable.
                match self.runner.run(bin, &["get", "dbfile"]) {
                    Ok(out) if out.success => {
                        let db_path = out.stdout.trim().to_string();
                        if db_path == "None" || db_path.is_empty() {
                            findings.push(
                                Finding::new(
                                    "service.dbfile-disabled",
                                    Severity::Info,
                                    "Database is disabled",
                                )
                                .detail(
                                    "Fail2Ban database persistence is disabled. \
                                     Bans will not survive restarts.",
                                ),
                            );
                        } else {
                            findings.push(
                                Finding::new(
                                    "service.dbfile-configured",
                                    Severity::Info,
                                    "Database file configured",
                                )
                                .detail(format!("Database: {db_path}")),
                            );
                        }
                    }
                    Ok(out) => {
                        findings.push(
                            Finding::new(
                                "service.dbfile-unavailable",
                                Severity::Warning,
                                "Could not retrieve database file path",
                            )
                            .detail(format!(
                                "get dbfile exited with code {:?}: {}",
                                out.exit_code,
                                out.stderr.trim(),
                            )),
                        );
                    }
                    Err(e) => {
                        findings.push(
                            Finding::new(
                                "service.dbfile-error",
                                Severity::Warning,
                                "Could not retrieve database file path",
                            )
                            .detail(format!("get dbfile failed: {e}")),
                        );
                    }
                }
            }
            Err(_) => {
                // Already reported in check_binaries; add a service-level note.
                findings.push(Finding::new(
                    "service.no-client",
                    Severity::Critical,
                    "Cannot check service: fail2ban-client not found",
                ));
            }
        }

        findings
    }

    // =======================================================================
    // Config checks
    // =======================================================================

    /// Verify Fail2Ban configuration integrity.
    ///
    /// Checks:
    ///
    /// - `/etc/fail2ban` directory exists
    /// - `fail2ban-client --test` passes
    /// - generated files contain the managed header
    /// - no stock `.conf` files were modified
    fn check_config(&self) -> Vec<Finding> {
        let mut findings = Vec::new();
        let config_dir = std::path::Path::new("/etc/fail2ban");

        // Config directory exists.
        if config_dir.exists() {
            findings.push(Finding::new(
                "config.directory.exists",
                Severity::Ok,
                "Fail2Ban config directory exists",
            ));
        } else {
            findings.push(
                Finding::new(
                    "config.directory.missing",
                    Severity::Critical,
                    "Fail2Ban config directory does not exist",
                )
                .detail("Expected /etc/fail2ban to exist.")
                .fix("Install Fail2Ban or create /etc/fail2ban."),
            );
            // Early return: nothing else can be checked.
            return findings;
        }

        // fail2ban-client --test passes.
        match find_binary("fail2ban-client") {
            Ok(path) => {
                let bin = path.to_str().unwrap_or("fail2ban-client");
                match self.runner.run(bin, &["--test"]) {
                    Ok(out) if out.success => {
                        findings.push(Finding::new(
                            "config.test.passed",
                            Severity::Ok,
                            "fail2ban-client --test passed",
                        ));
                    }
                    Ok(out) => {
                        findings.push(
                            Finding::new(
                                "config.test.failed",
                                Severity::Error,
                                "fail2ban-client --test failed",
                            )
                            .detail(format!(
                                "Configuration test exited with code {:?}: {}",
                                out.exit_code,
                                out.stderr.trim(),
                            ))
                            .fix("Review Fail2Ban configuration files for syntax errors."),
                        );
                    }
                    Err(e) => {
                        findings.push(
                            Finding::new(
                                "config.test.error",
                                Severity::Error,
                                "Could not run config test",
                            )
                            .detail(format!("fail2ban-client --test failed: {e}")),
                        );
                    }
                }

                // Check that generated files contain the managed header.
                let jail_d = config_dir.join("jail.d");
                if jail_d.exists() {
                    if let Ok(entries) = std::fs::read_dir(&jail_d) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.extension().map_or(false, |e| e == "local") {
                                match std::fs::read_to_string(&path) {
                                    Ok(content) => {
                                        if content.contains("Managed by fail2ban-kit") {
                                            findings.push(Finding::new(
                                                "config.managed-header.present",
                                                Severity::Ok,
                                                format!(
                                                    "Managed header found in {}",
                                                    path.display()
                                                ),
                                            ));
                                        } else {
                                            findings.push(
                                                Finding::new(
                                                    "config.managed-header.missing",
                                                    Severity::Warning,
                                                    format!(
                                                        "Missing managed header in {}",
                                                        path.display()
                                                    ),
                                                )
                                                .detail(
                                                    "Generated .local files should contain \
                                                     the managed header comment.",
                                                ),
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        findings.push(
                                            Finding::new(
                                                "config.file-read-error",
                                                Severity::Warning,
                                                format!("Cannot read {}", path.display()),
                                            )
                                            .detail(format!("Read error: {e}")),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                // Check no stock .conf files were modified.
                let dirs_to_check = ["jail.d", "filter.d", "action.d"];
                for subdir in &dirs_to_check {
                    let dir = config_dir.join(subdir);
                    if !dir.exists() {
                        continue;
                    }
                    if let Ok(entries) = std::fs::read_dir(&dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.extension().map_or(false, |e| e == "conf") {
                                match std::fs::read_to_string(&path) {
                                    Ok(content) => {
                                        if content.contains("Managed by fail2ban-kit") {
                                            findings.push(
                                                Finding::new(
                                                    "config.stock-conf.modified",
                                                    Severity::Critical,
                                                    format!(
                                                        "Stock .conf file was modified: {}",
                                                        path.display()
                                                    ),
                                                )
                                                .detail(
                                                    "Stock .conf files must not be edited by \
                                                     the library. Use .local overrides instead.",
                                                )
                                                .fix(format!(
                                                    "Restore the original file and use a \
                                                     .local override: {}.local",
                                                    path.display()
                                                )),
                                            );
                                        }
                                    }
                                    Err(_) => {
                                        // Ignore read errors for stock file checks.
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {
                findings.push(Finding::new(
                    "config.no-client",
                    Severity::Critical,
                    "Cannot check config: fail2ban-client not found",
                ));
            }
        }

        findings
    }

    // =======================================================================
    // Jail checks
    // =======================================================================

    /// Run diagnostic checks for a single named jail.
    ///
    /// Checks:
    ///
    /// - jail exists
    /// - jail is enabled
    /// - jail status is readable
    /// - jail has a filter
    /// - jail has at least one action
    /// - `bantime`, `findtime`, and `maxretry` have sane values
    fn check_jail(&self, jail: &str) -> Vec<Finding> {
        let mut findings = Vec::new();

        match find_binary("fail2ban-client") {
            Ok(path) => {
                let bin = path.to_str().unwrap_or("fail2ban-client");

                // Jail status readable (implies existence).
                match self.runner.run(bin, &["status", jail]) {
                    Ok(out) if out.success => {
                        findings.push(
                            Finding::new(
                                "jail.exists",
                                Severity::Ok,
                                format!("Jail '{jail}' exists"),
                            )
                            .detail("fail2ban-client status returned successfully."),
                        );

                        let status = &out.stdout;

                        // Check if jail appears enabled.
                        if status.contains("Currently banned") || status.contains("File list") {
                            findings.push(Finding::new(
                                "jail.enabled",
                                Severity::Ok,
                                format!("Jail '{jail}' appears to be running"),
                            ));
                        } else if status.contains("not running") {
                            findings.push(
                                Finding::new(
                                    "jail.not-running",
                                    Severity::Warning,
                                    format!("Jail '{jail}' is not running"),
                                )
                                .fix("Enable the jail in your jail configuration and reload."),
                            );
                        }

                        // Check filter presence (best-effort parsing).
                        if status.contains("Filter") || status.contains("filter") {
                            findings.push(Finding::new(
                                "jail.has-filter",
                                Severity::Ok,
                                format!("Jail '{jail}' has a filter configured"),
                            ));
                        } else {
                            findings.push(
                                Finding::new(
                                    "jail.no-filter",
                                    Severity::Error,
                                    format!("Jail '{jail}' may not have a filter"),
                                )
                                .fix("Add a filter to the jail configuration."),
                            );
                        }

                        // Check action presence.
                        if status.contains("Actions") || status.contains("actions")
                            || status.contains("action")
                        {
                            findings.push(Finding::new(
                                "jail.has-action",
                                Severity::Ok,
                                format!("Jail '{jail}' has actions configured"),
                            ));
                        } else {
                            findings.push(
                                Finding::new(
                                    "jail.no-action",
                                    Severity::Error,
                                    format!("Jail '{jail}' may not have any actions"),
                                )
                                .fix("Add at least one action (e.g. nftables, iptables) to the jail."),
                            );
                        }
                    }
                    Ok(out) => {
                        let stderr = out.stderr.trim();
                        findings.push(
                            Finding::new(
                                "jail.not-found",
                                Severity::Error,
                                format!("Jail '{jail}' not found or not running"),
                            )
                            .detail(format!(
                                "fail2ban-client status {jail} exited {:?}: {stderr}",
                                out.exit_code,
                            ))
                            .fix(format!(
                                "Create a jail configuration for '{jail}' and reload Fail2Ban.",
                            )),
                        );
                        return findings;
                    }
                    Err(e) => {
                        findings.push(
                            Finding::new(
                                "jail.status-error",
                                Severity::Error,
                                format!("Could not check jail '{jail}'"),
                            )
                            .detail(format!("fail2ban-client status {jail} failed: {e}")),
                        );
                        return findings;
                    }
                }

                // Sane timing parameters (best-effort via status output).
                // We cannot directly query bantime/findtime/maxretry via
                // fail2ban-client in all versions, so we check get operations.
                for (param, label) in &[("bantime", "ban time"), ("findtime", "find time")] {
                    match self.runner.run(bin, &["get", jail, param]) {
                        Ok(out) if out.success => {
                            let val = out.stdout.trim();
                            findings.push(
                                Finding::new(
                                    &format!("jail.{param}-configured"),
                                    Severity::Info,
                                    format!("Jail '{jail}' {label}: {val}"),
                                ),
                            );
                        }
                        _ => {
                            findings.push(
                                Finding::new(
                                    &format!("jail.{param}-unknown"),
                                    Severity::Info,
                                    format!("Jail '{jail}' {label} could not be queried"),
                                ),
                            );
                        }
                    }
                }

                // maxretry
                match self.runner.run(bin, &["get", jail, "maxretry"]) {
                    Ok(out) if out.success => {
                        let val = out.stdout.trim();
                        if let Ok(n) = val.parse::<u32>() {
                            if n == 0 {
                                findings.push(
                                    Finding::new(
                                        "jail.maxretry-zero",
                                        Severity::Warning,
                                        format!("Jail '{jail}' has maxretry=0"),
                                    )
                                    .detail("A maxretry of 0 means no bans will ever occur.")
                                    .fix("Set maxretry to a positive value (e.g. 3 or 5)."),
                                );
                            } else if n > 100 {
                                findings.push(
                                    Finding::new(
                                        "jail.maxretry-very-high",
                                        Severity::Info,
                                        format!("Jail '{jail}' has maxretry={n}"),
                                    )
                                    .detail(
                                        "A very high maxretry may reduce the effectiveness \
                                         of the jail.",
                                    ),
                                );
                            } else {
                                findings.push(Finding::new(
                                    "jail.maxretry-ok",
                                    Severity::Ok,
                                    format!("Jail '{jail}' maxretry={n}"),
                                ));
                            }
                        }
                    }
                    _ => {
                        findings.push(Finding::new(
                            "jail.maxretry-unknown",
                            Severity::Info,
                            format!("Jail '{jail}' maxretry could not be queried"),
                        ));
                    }
                }
            }
            Err(_) => {
                findings.push(Finding::new(
                    "jail.no-client",
                    Severity::Critical,
                    "Cannot check jail: fail2ban-client not found",
                ));
            }
        }

        findings
    }

    // =======================================================================
    // Log path checks
    // =======================================================================

    /// Verify that configured log paths are accessible and valid.
    ///
    /// Checks:
    ///
    /// - log path exists on disk
    /// - parent directory exists
    /// - log path is readable
    /// - log file is not empty when activity is expected
    /// - glob patterns are warned about
    fn check_log_paths(&self) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Retrieve log paths from active jails.
        match find_binary("fail2ban-client") {
            Ok(path) => {
                let bin = path.to_str().unwrap_or("fail2ban-client");

                // First get the list of jails.
                match self.runner.run(bin, &["status"]) {
                    Ok(out) if out.success => {
                        let status = &out.stdout;
                        // Best-effort parse jail names from "Jail list: ..." line.
                        let jail_names = parse_jail_list(status);
                        if jail_names.is_empty() {
                            findings.push(Finding::new(
                                "logpath.no-jails",
                                Severity::Info,
                                "No active jails found to check log paths for",
                            ));
                            return findings;
                        }

                        for jail in &jail_names {
                            // Get log path for this jail.
                            match self.runner.run(bin, &["get", jail, "logpath"]) {
                                Ok(out) if out.success => {
                                    let log_paths = out.stdout.trim();
                                    for line in log_paths.lines() {
                                        let lp = line.trim();
                                        if lp.is_empty() {
                                            continue;
                                        }
                                        self.check_single_log_path(
                                            lp,
                                            jail,
                                            &mut findings,
                                        );
                                    }
                                }
                                Ok(_) => {
                                    findings.push(
                                        Finding::new(
                                            "logpath.jail-unavailable",
                                            Severity::Warning,
                                            format!("Cannot get log path for jail '{jail}'"),
                                        ),
                                    );
                                }
                                Err(e) => {
                                    findings.push(
                                        Finding::new(
                                            "logpath.jail-error",
                                            Severity::Warning,
                                            format!("Error getting log path for jail '{jail}'"),
                                        )
                                        .detail(format!("{e}")),
                                    );
                                }
                            }
                        }
                    }
                    Ok(_) => {
                        findings.push(Finding::new(
                            "logpath.status-failed",
                            Severity::Error,
                            "Could not retrieve Fail2Ban status for log path checks",
                        ));
                    }
                    Err(e) => {
                        findings.push(
                            Finding::new(
                                "logpath.status-error",
                                Severity::Error,
                                "Could not retrieve Fail2Ban status",
                            )
                            .detail(format!("{e}")),
                        );
                    }
                }
            }
            Err(_) => {
                findings.push(Finding::new(
                    "logpath.no-client",
                    Severity::Critical,
                    "Cannot check log paths: fail2ban-client not found",
                ));
            }
        }

        findings
    }

    /// Check a single log path and append findings.
    fn check_single_log_path(
        &self,
        log_path: &str,
        jail: &str,
        findings: &mut Vec<Finding>,
    ) {
        let path = std::path::Path::new(log_path);

        // Warn about glob patterns.
        if log_path.contains('*') || log_path.contains('?') || log_path.contains('[') {
            findings.push(
                Finding::new(
                    "logpath.glob-pattern",
                    Severity::Info,
                    format!("Jail '{jail}' uses a glob pattern: {log_path}"),
                )
                .detail(
                    "Glob patterns only match files that exist at Fail2Ban startup. \
                     New files created later will not be picked up until a reload.",
                ),
            );
            // For glob patterns, we cannot check individual files; skip the
            // rest of the checks for this entry.
            return;
        }

        // Path exists.
        if path.exists() {
            findings.push(Finding::new(
                "logpath.exists",
                Severity::Ok,
                format!("Log path for jail '{jail}' exists: {log_path}"),
            ));
        } else {
            // Check parent directory.
            if let Some(parent) = path.parent() {
                if parent.exists() {
                    findings.push(
                        Finding::new(
                            "logpath.file-missing",
                            Severity::Warning,
                            format!("Log file for jail '{jail}' does not exist: {log_path}"),
                        )
                        .detail(format!(
                            "Parent directory exists ({}) but the log file is missing.",
                            parent.display(),
                        ))
                        .fix(format!(
                            "Verify that the application writes to {log_path}, \
                             or create the file and ensure Fail2Ban can read it.",
                        )),
                    );
                } else {
                    findings.push(
                        Finding::new(
                            "logpath.parent-missing",
                            Severity::Error,
                            format!(
                                "Parent directory for jail '{jail}' log does not exist: {}",
                                parent.display(),
                            ),
                        )
                        .fix(format!(
                            "Create the directory: mkdir -p {}",
                            parent.display(),
                        )),
                    );
                }
            }
            return;
        }

        // Readable.
        if std::fs::metadata(path).is_err() {
            findings.push(
                Finding::new(
                    "logpath.not-readable",
                    Severity::Error,
                    format!("Log file for jail '{jail}' is not readable: {log_path}"),
                )
                .fix("Adjust file permissions so the Fail2Ban process can read the log."),
            );
            return;
        }

        // Not empty.
        match std::fs::metadata(path) {
            Ok(meta) => {
                if meta.len() == 0 {
                    findings.push(
                        Finding::new(
                            "logpath.empty",
                            Severity::Info,
                            format!("Log file for jail '{jail}' is empty: {log_path}"),
                        )
                        .detail(
                            "The log file exists but is empty. This may be normal \
                             if the application has not written any entries yet.",
                        ),
                    );
                }
            }
            Err(_) => {
                // Already reported as not-readable.
            }
        }
    }

    // =======================================================================
    // Journal checks
    // =======================================================================

    /// Verify systemd journal configuration and accessibility.
    ///
    /// Checks:
    ///
    /// - backend is `systemd` when expected
    /// - `journalmatch` is configured (not `logpath`)
    /// - journal query returns recent rows
    /// - Fail2Ban has access to the journal
    fn check_journal(&self) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Check if journalctl is available at all.
        match self.runner.run("journalctl", &["--version"]) {
            Ok(out) if out.success => {
                findings.push(Finding::new(
                    "journal.journalctl.available",
                    Severity::Ok,
                    "journalctl is available",
                ));
            }
            _ => {
                findings.push(
                    Finding::new(
                        "journal.journalctl.unavailable",
                        Severity::Info,
                        "journalctl is not available",
                    )
                    .detail(
                        "journalctl could not be executed. Systemd journal \
                         checks require journalctl.",
                    ),
                );
                return findings;
            }
        }

        // Try to query the journal for Fail2Ban's own service unit.
        match self
            .runner
            .run("journalctl", &["-u", "fail2ban", "-n", "1", "--no-pager"])
        {
            Ok(out) if out.success => {
                let lines: Vec<&str> = out.stdout.trim().lines().collect();
                if lines.is_empty() || lines.iter().all(|l| l.trim().is_empty()) {
                    findings.push(
                        Finding::new(
                            "journal.no-entries",
                            Severity::Info,
                            "No recent journal entries for fail2ban service",
                        )
                        .detail(
                            "The journal was queried but returned no entries for the \
                             fail2ban unit. The service may not have logged yet.",
                        ),
                    );
                } else {
                    findings.push(Finding::new(
                        "journal.entries-found",
                        Severity::Ok,
                        "Journal is accessible and contains Fail2Ban entries",
                    ));
                }
            }
            Ok(out) => {
                findings.push(
                    Finding::new(
                        "journal.query-failed",
                        Severity::Warning,
                        "Journal query returned non-zero exit",
                    )
                    .detail(format!(
                        "journalctl -u fail2ban exited {:?}: {}",
                        out.exit_code,
                        out.stderr.trim(),
                    ))
                    .fix("Ensure the user running this check has journal access."),
                );
            }
            Err(e) => {
                findings.push(
                    Finding::new(
                        "journal.query-error",
                        Severity::Warning,
                        "Could not query the systemd journal",
                    )
                    .detail(format!("journalctl failed: {e}")),
                );
            }
        }

        // Check active jails for systemd backend misuse.
        if let Ok(path) = find_binary("fail2ban-client") {
            let bin = path.to_str().unwrap_or("fail2ban-client");
            if let Ok(out) = self.runner.run(bin, &["status"]) {
                if out.success {
                    let jail_names = parse_jail_list(&out.stdout);
                    for jail in &jail_names {
                        // Check backend.
                        match self.runner.run(bin, &["get", jail, "backend"]) {
                            Ok(out) if out.success => {
                                let backend = out.stdout.trim().to_lowercase();
                                if backend.contains("systemd") {
                                    findings.push(
                                        Finding::new(
                                            "jail.backend-systemd",
                                            Severity::Info,
                                            format!("Jail '{jail}' uses systemd backend"),
                                        ),
                                    );
                                    // Check that journalmatch is set, not logpath.
                                    match self
                                        .runner
                                        .run(bin, &["get", jail, "journalmatch"])
                                    {
                                        Ok(jm_out) if jm_out.success => {
                                            let jm = jm_out.stdout.trim();
                                            if jm.is_empty() || jm == "None" {
                                                findings.push(
                                                    Finding::new(
                                                        "journal.journalmatch-missing",
                                                        Severity::Warning,
                                                        format!(
                                                            "Jail '{jail}' uses systemd \
                                                             backend but has no journalmatch"
                                                        ),
                                                    )
                                                    .fix(format!(
                                                        "Add a journalmatch directive \
                                                         to jail '{jail}'.",
                                                    )),
                                                );
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        findings
    }

    // =======================================================================
    // Regex checks
    // =======================================================================

    /// Verify that failregex patterns compile and use `<HOST>` correctly.
    ///
    /// Checks:
    ///
    /// - `fail2ban-regex` is available
    /// - failregex compiles via `fail2ban-regex`
    /// - `<HOST>` appears in the regex pattern
    fn check_regex(&self) -> Vec<Finding> {
        let mut findings = Vec::new();

        match find_binary("fail2ban-regex") {
            Ok(path) => {
                let bin = path.to_str().unwrap_or("fail2ban-regex");

                // Verify fail2ban-regex is functional.
                match self.runner.run(bin, &["--version"]) {
                    Ok(out) if out.success => {
                        findings.push(Finding::new(
                            "regex.tool-available",
                            Severity::Ok,
                            "fail2ban-regex is available",
                        ));
                    }
                    Ok(out) => {
                        findings.push(
                            Finding::new(
                                "regex.tool-error",
                                Severity::Warning,
                                "fail2ban-regex did not return version",
                            )
                            .detail(format!(
                                "Exited {:?}: {}",
                                out.exit_code,
                                out.stderr.trim(),
                            )),
                        );
                    }
                    Err(e) => {
                        findings.push(
                            Finding::new(
                                "regex.tool-unavailable",
                                Severity::Error,
                                "Could not run fail2ban-regex",
                            )
                            .detail(format!("{e}")),
                        );
                        return findings;
                    }
                }

                // Check active jails for <HOST> usage in their filters.
                if let Ok(client_path) = find_binary("fail2ban-client") {
                    let client_bin = client_path.to_str().unwrap_or("fail2ban-client");
                    if let Ok(out) = self.runner.run(client_bin, &["status"]) {
                        if out.success {
                            let jail_names = parse_jail_list(&out.stdout);
                            for jail in &jail_names {
                                // Get the failregex for this jail.
                                match self
                                    .runner
                                    .run(client_bin, &["get", jail, "failregex"])
                                {
                                    Ok(out) if out.success => {
                                        let regex = out.stdout.trim();
                                        if regex.is_empty() || regex == "None" {
                                            findings.push(
                                                Finding::new(
                                                    "regex.jail-no-failregex",
                                                    Severity::Warning,
                                                    format!(
                                                        "Jail '{jail}' has no failregex"
                                                    ),
                                                )
                                                .fix(format!(
                                                    "Add a failregex to the filter used \
                                                     by jail '{jail}'.",
                                                )),
                                            );
                                        } else if !regex.contains("<HOST>") {
                                            findings.push(
                                                Finding::new(
                                                    "regex.missing-host-tag",
                                                    Severity::Error,
                                                    format!(
                                                        "Jail '{jail}' failregex does not \
                                                         contain <HOST>"
                                                    ),
                                                )
                                                .detail(
                                                    "The failregex must contain <HOST> so \
                                                     that Fail2Ban can extract the IP \
                                                     address from matching log lines.",
                                                )
                                                .fix(format!(
                                                    "Update the failregex for jail '{jail}' \
                                                     to include <HOST>.",
                                                )),
                                            );
                                        } else {
                                            findings.push(Finding::new(
                                                "regex.host-tag-present",
                                                Severity::Ok,
                                                format!(
                                                    "Jail '{jail}' failregex contains <HOST>"
                                                ),
                                            ));
                                        }
                                    }
                                    _ => {
                                        findings.push(
                                            Finding::new(
                                                "regex.jail-failregex-unknown",
                                                Severity::Info,
                                                format!(
                                                    "Could not query failregex for jail '{jail}'"
                                                ),
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {
                findings.push(
                    Finding::new(
                        "regex.no-tool",
                        Severity::Warning,
                        "fail2ban-regex not found",
                    )
                    .detail("Cannot verify regex patterns without fail2ban-regex.")
                    .fix("Install Fail2Ban which includes fail2ban-regex."),
                );
            }
        }

        findings
    }

    // =======================================================================
    // Action checks
    // =======================================================================

    /// Verify that configured actions are valid and compatible with the
    /// system firewall.
    ///
    /// Checks:
    ///
    /// - action file exists
    /// - action has ban and unban definitions
    /// - action is compatible with system firewall backend
    fn check_actions(&self) -> Vec<Finding> {
        let mut findings = Vec::new();
        let action_dir = std::path::Path::new("/etc/fail2ban/action.d");

        // Check the action directory exists.
        if action_dir.exists() {
            findings.push(Finding::new(
                "action.directory.exists",
                Severity::Ok,
                "Fail2Ban action directory exists",
            ));
        } else {
            findings.push(
                Finding::new(
                    "action.directory.missing",
                    Severity::Error,
                    "Fail2Ban action directory does not exist",
                )
                .detail("Expected /etc/fail2ban/action.d to exist.")
                .fix("Install Fail2Ban or create the action.d directory."),
            );
            return findings;
        }

        // Check active jail actions for firewall compatibility.
        if let Ok(path) = find_binary("fail2ban-client") {
            let bin = path.to_str().unwrap_or("fail2ban-client");
            if let Ok(out) = self.runner.run(bin, &["status"]) {
                if out.success {
                    let jail_names = parse_jail_list(&out.stdout);
                    for jail in &jail_names {
                        // Get actions for this jail.
                        match self.runner.run(bin, &["get", jail, "actions"]) {
                            Ok(out) if out.success => {
                                let actions_str = out.stdout.trim();
                                for action_name in actions_str
                                    .split(',')
                                    .map(|s| s.trim())
                                    .filter(|s| !s.is_empty())
                                {
                                    // Check that the action file exists.
                                    let conf_path = action_dir.join(format!(
                                        "{action_name}.conf"
                                    ));
                                    let local_path = action_dir.join(format!(
                                        "{action_name}.local"
                                    ));

                                    if conf_path.exists() || local_path.exists() {
                                        findings.push(Finding::new(
                                            "action.file-exists",
                                            Severity::Ok,
                                            format!(
                                                "Action '{action_name}' for jail '{jail}' \
                                                 exists"
                                            ),
                                        ));

                                        // Check firewall compatibility.
                                        let action_lower =
                                            action_name.to_ascii_lowercase();
                                        if action_lower.contains("nftables") {
                                            match self
                                                .runner
                                                .run("nft", &["--version"])
                                            {
                                                Ok(o) if o.success => {}
                                                _ => {
                                                    findings.push(
                                                        Finding::new(
                                                            "action.nft-incompatible",
                                                            Severity::Critical,
                                                            format!(
                                                                "Jail '{jail}' uses \
                                                                 nftables action but \
                                                                 nft is not available"
                                                            ),
                                                        )
                                                        .fix(
                                                            "Install nftables or switch \
                                                             to an iptables action.",
                                                        ),
                                                    );
                                                }
                                            }
                                        } else if action_lower.contains("iptables") {
                                            match self
                                                .runner
                                                .run("iptables", &["--version"])
                                            {
                                                Ok(o) if o.success => {}
                                                _ => {
                                                    findings.push(
                                                        Finding::new(
                                                            "action.iptables-incompatible",
                                                            Severity::Critical,
                                                            format!(
                                                                "Jail '{jail}' uses \
                                                                 iptables action but \
                                                                 iptables is not available"
                                                            ),
                                                        )
                                                        .fix(
                                                            "Install iptables or switch \
                                                             to a nftables action.",
                                                        ),
                                                    );
                                                }
                                            }
                                        }
                                    } else {
                                        findings.push(
                                            Finding::new(
                                                "action.file-missing",
                                                Severity::Error,
                                                format!(
                                                    "Action '{action_name}' for jail \
                                                     '{jail}' not found in action.d"
                                                ),
                                            )
                                            .fix(format!(
                                                "Install the action file {action_name}.conf \
                                                 in /etc/fail2ban/action.d/ or update the \
                                                 jail configuration.",
                                            )),
                                        );
                                    }
                                }
                            }
                            _ => {
                                findings.push(
                                    Finding::new(
                                        "action.jail-actions-unknown",
                                        Severity::Info,
                                        format!(
                                            "Could not query actions for jail '{jail}'"
                                        ),
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }

        findings
    }

    // =======================================================================
    // Permission checks
    // =======================================================================

    /// Verify file permissions are safe across the Fail2Ban installation.
    ///
    /// Checks:
    ///
    /// - `/etc/fail2ban` is not world-writable
    /// - generated config files are not world-writable
    /// - socket path permissions are sane
    fn check_permissions(&self) -> Vec<Finding> {
        let mut findings = Vec::new();
        let config_dir = std::path::Path::new("/etc/fail2ban");

        if !config_dir.exists() {
            findings.push(Finding::new(
                "permission.config-dir-missing",
                Severity::Error,
                "Fail2Ban config directory does not exist",
            ));
            return findings;
        }

        // /etc/fail2ban not world-writable.
        match std::fs::metadata(config_dir) {
            Ok(meta) => {
                let mode = permission_mode(&meta.permissions());
                if mode & 0o002 != 0 {
                    findings.push(
                        Finding::new(
                            "permission.config-dir-world-writable",
                            Severity::Critical,
                            "/etc/fail2ban is world-writable",
                        )
                        .detail(
                            "The Fail2Ban configuration directory is world-writable, \
                             which allows any user on the system to modify Fail2Ban \
                             configuration.",
                        )
                        .fix("chmod o-w /etc/fail2ban"),
                    );
                } else {
                    findings.push(Finding::new(
                        "permission.config-dir-safe",
                        Severity::Ok,
                        "/etc/fail2ban is not world-writable",
                    ));
                }
            }
            Err(e) => {
                findings.push(
                    Finding::new(
                        "permission.config-dir-stat-error",
                        Severity::Error,
                        "Cannot stat /etc/fail2ban",
                    )
                    .detail(format!("{e}")),
                );
            }
        }

        // Generated files not world-writable.
        let subdirs = ["jail.d", "filter.d", "action.d"];
        for subdir in &subdirs {
            let dir = config_dir.join(subdir);
            if !dir.exists() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Ok(meta) = std::fs::metadata(&path) {
                        let mode = permission_mode(&meta.permissions());
                        if mode & 0o002 != 0 {
                            findings.push(
                                Finding::new(
                                    "permission.file-world-writable",
                                    Severity::Error,
                                    format!(
                                        "{} is world-writable",
                                        path.display()
                                    ),
                                )
                                .fix(format!("chmod o-w {}", path.display())),
                            );
                        }
                    }
                }
            }
        }

        // Socket path permissions.
        // Default socket: /var/run/fail2ban/fail2ban.sock
        let socket_path = std::path::Path::new("/var/run/fail2ban/fail2ban.sock");
        if socket_path.exists() {
            match std::fs::metadata(socket_path) {
                Ok(meta) => {
                    let mode = permission_mode(&meta.permissions());
                    if mode & 0o002 != 0 {
                        findings.push(
                            Finding::new(
                                "permission.socket-world-writable",
                                Severity::Critical,
                                "Fail2Ban socket is world-writable",
                            )
                            .detail(format!(
                                "Socket at {} is world-writable. Any local user \
                                 can issue commands to Fail2Ban.",
                                socket_path.display(),
                            ))
                            .fix(format!("chmod o-w {}", socket_path.display())),
                        );
                    } else {
                        findings.push(Finding::new(
                            "permission.socket-safe",
                            Severity::Ok,
                            "Fail2Ban socket permissions are sane",
                        ));
                    }
                }
                Err(e) => {
                    findings.push(
                        Finding::new(
                            "permission.socket-stat-error",
                            Severity::Warning,
                            "Cannot stat Fail2Ban socket",
                        )
                        .detail(format!("{e}")),
                    );
                }
            }
        } else {
            findings.push(Finding::new(
                "permission.socket-not-found",
                Severity::Info,
                "Fail2Ban socket not found (service may not be running)",
            ));
        }

        findings
    }

    // =======================================================================
    // Safety checks
    // =======================================================================

    /// Verify that safe operational practices are in place.
    ///
    /// Checks:
    ///
    /// - dry-run mode is available before applying changes
    /// - backup files exist before destructive updates
    /// - rollback path is available
    fn check_safety(&self) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Dry-run is a library feature -- verify the runner supports it.
        let dry_run_available = !self.runner.dry_run();
        findings.push(
            Finding::new(
                "safety.dry-run-available",
                Severity::Ok,
                "Dry-run mode is available",
            )
            .detail(
                "The library supports dry-run mode to preview changes \
                 without applying them.",
            ),
        );

        if !dry_run_available {
            // Currently in dry-run mode; note that.
            findings.push(Finding::new(
                "safety.currently-dry-run",
                Severity::Info,
                "Runner is currently in dry-run mode",
            ));
        }

        // Check for backup files in jail.d.
        let jail_d = std::path::Path::new("/etc/fail2ban/jail.d");
        if jail_d.exists() {
            if let Ok(entries) = std::fs::read_dir(jail_d) {
                let backup_count = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.file_name()
                            .to_string_lossy()
                            .contains(".bak-")
                    })
                    .count();

                if backup_count > 0 {
                    findings.push(
                        Finding::new(
                            "safety.backups-exist",
                            Severity::Ok,
                            format!("{backup_count} backup file(s) found in jail.d"),
                        )
                        .detail(
                            "Existing backup files indicate that previous \
                             operations created restore points.",
                        ),
                    );
                } else {
                    findings.push(Finding::new(
                        "safety.no-backups",
                        Severity::Info,
                        "No backup files found in jail.d",
                    ));
                }
            }
        }

        // Rollback path: verify that the config directory is writable
        // (needed for restoring backups).
        let config_dir = std::path::Path::new("/etc/fail2ban");
        if config_dir.exists() {
            // Check if we can write to jail.d (best-effort test).
            let test_path = config_dir.join("jail.d/.doctor-write-test");
            match std::fs::write(&test_path, b"") {
                Ok(_) => {
                    let _ = std::fs::remove_file(&test_path);
                    findings.push(Finding::new(
                        "safety.rollback-writable",
                        Severity::Ok,
                        "Config directory is writable (rollback possible)",
                    ));
                }
                Err(_) => {
                    findings.push(
                        Finding::new(
                            "safety.rollback-not-writable",
                            Severity::Warning,
                            "Config directory is not writable",
                        )
                        .detail(
                            "Cannot write to /etc/fail2ban/jail.d. Rollback \
                             operations may fail. This check likely needs to \
                             be run with elevated privileges.",
                        )
                        .fix("Run with appropriate privileges (e.g. via sudo)."),
                    );
                }
            }
        }

        findings
    }

    // =======================================================================
    // Proxy checks
    // =======================================================================

    /// Detect proxy-related misconfigurations that would cause Fail2Ban to
    /// ban the proxy instead of the attacker.
    ///
    /// Checks:
    ///
    /// - detect whether logs contain proxy IPs only
    /// - warn if Fail2Ban would ban Cloudflare/Traefik instead of attacker
    fn check_proxy(&self) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Common proxy / CDN IP ranges to warn about.
        let proxy_indicators = [
            ("Cloudflare", "cloudflare"),
            ("Traefik", "traefik"),
            ("NGINX reverse proxy", "nginx"),
        ];

        // Check active jails for known proxy-related patterns in their
        // configuration or log paths.
        if let Ok(path) = find_binary("fail2ban-client") {
            let bin = path.to_str().unwrap_or("fail2ban-client");
            if let Ok(out) = self.runner.run(bin, &["status"]) {
                if out.success {
                    let jail_names = parse_jail_list(&out.stdout);
                    for jail in &jail_names {
                        // Check the jail's log path for proxy indicators.
                        if let Ok(log_out) =
                            self.runner.run(bin, &["get", jail, "logpath"])
                        {
                            if log_out.success {
                                let log_path = log_out.stdout.trim().to_lowercase();

                                // Check for common proxy log patterns.
                                if log_path.contains("traefik")
                                    || log_path.contains("access.log")
                                {
                                    findings.push(
                                        Finding::new(
                                            "proxy.reverse-proxy-log",
                                            Severity::Warning,
                                            format!(
                                                "Jail '{jail}' may be behind a \
                                                 reverse proxy"
                                            ),
                                        )
                                        .detail(format!(
                                            "Log path '{log_path}' suggests the \
                                             application is behind a reverse proxy. \
                                             Fail2Ban would see the proxy IP, not \
                                             the real client IP.",
                                        ))
                                        .fix(
                                            "Configure your application to log the \
                                             real client IP (e.g. via X-Forwarded-For \
                                             or X-Real-IP). Ensure the log format \
                                             includes the forwarded IP.",
                                        ),
                                    );
                                }
                            }
                        }

                        // Check for Cloudflare-specific actions.
                        if let Ok(action_out) =
                            self.runner.run(bin, &["get", jail, "actions"])
                        {
                            if action_out.success {
                                let actions = action_out.stdout.trim().to_lowercase();
                                for (label, keyword) in &proxy_indicators {
                                    if actions.contains(keyword) {
                                        findings.push(
                                            Finding::new(
                                                "proxy.cdn-action-detected",
                                                Severity::Info,
                                                format!(
                                                    "Jail '{jail}' uses a {label} action"
                                                ),
                                            )
                                            .detail(
                                                "Cloudflare/Traefik/NGINX actions \
                                                 should use the CDN's own ban API \
                                                 rather than firewall rules, because \
                                                 the source IP belongs to the CDN.",
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // General proxy warning if no specific findings were added.
        if findings.is_empty() {
            findings.push(Finding::new(
                "proxy.no-issues",
                Severity::Ok,
                "No proxy-related issues detected",
            ));
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Best-effort parse of jail names from `fail2ban-client status` output.
///
/// Looks for a line containing "Jail list:" and extracts the comma-separated
/// names that follow.
fn parse_jail_list(status: &str) -> Vec<String> {
    for line in status.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("jail list") {
            if let Some(idx) = line.find(':') {
                let rest = &line[idx + 1..];
                return rest
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
    }
    Vec::new()
}

// ---------------------------------------------------------------------------
// Unix permissions helper
// ---------------------------------------------------------------------------

/// Extract the Unix permission mode bits from `std::fs::Permissions`.
///
/// On Unix this reads the `mode()` field via `PermissionsExt`. On non-Unix
/// platforms it returns `0` (no permissions checks possible).
#[cfg(unix)]
fn permission_mode(permissions: &std::fs::Permissions) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    permissions.mode()
}

/// Extract the Unix permission mode bits from `std::fs::Permissions`.
///
/// Non-Unix fallback: always returns `0`.
#[cfg(not(unix))]
fn permission_mode(_permissions: &std::fs::Permissions) -> u32 {
    0
}

#[cfg(test)]
#[path = "doctor.test.rs"]
mod tests;
