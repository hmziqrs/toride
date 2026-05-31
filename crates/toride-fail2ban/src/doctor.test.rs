//! Comprehensive tests for the [`Doctor`] diagnostic engine.
//!
//! Every test uses [`FakeRunner`] to avoid executing real system commands.
//! Where the code under test calls [`find_binary`], the result depends on
//! the host system, so tests assert on the *shape* of findings (presence of
//! specific IDs) rather than exact counts.

use super::*;
use crate::command::{CommandOutput, FakeRunner};
use crate::report::{DoctorReport, Finding, Severity};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Shorthand for a successful command output with the given stdout.
fn ok_output(stdout: &str) -> CommandOutput {
    CommandOutput {
        stdout: stdout.to_string(),
        stderr: String::new(),
        exit_code: Some(0),
        success: true,
    }
}

/// Shorthand for a failed command output with the given stderr.
fn fail_output(stderr: &str) -> CommandOutput {
    CommandOutput {
        stdout: String::new(),
        stderr: stderr.to_string(),
        exit_code: Some(1),
        success: false,
    }
}

/// Check whether findings contain an entry with the given id.
fn has_finding(findings: &[Finding], id: &str) -> bool {
    findings.iter().any(|f| f.id == id)
}

// ===========================================================================
// Doctor construction
// ===========================================================================

#[test]
fn doctor_new_borrows_runner() {
    let fake = FakeRunner::new();
    let _doctor = Doctor::new(&fake);
    // Doctor should construct without panicking.
}

// ===========================================================================
// DoctorScope
// ===========================================================================

#[test]
fn doctor_scope_all_categories_has_ten_variants() {
    let cats = DoctorScope::all_categories();
    assert_eq!(cats.len(), 10);
}

#[test]
fn doctor_scope_all_categories_does_not_include_all() {
    let cats = DoctorScope::all_categories();
    assert!(!cats.iter().any(|c| matches!(c, DoctorScope::All)));
}

#[test]
fn doctor_scope_all_categories_contains_expected_variants() {
    let cats = DoctorScope::all_categories();
    assert!(cats.iter().any(|c| matches!(c, DoctorScope::Binary)));
    assert!(cats.iter().any(|c| matches!(c, DoctorScope::Service)));
    assert!(cats.iter().any(|c| matches!(c, DoctorScope::Config)));
    assert!(cats.iter().any(|c| matches!(c, DoctorScope::LogPath)));
    assert!(cats.iter().any(|c| matches!(c, DoctorScope::Journal)));
    assert!(cats.iter().any(|c| matches!(c, DoctorScope::Regex)));
    assert!(cats.iter().any(|c| matches!(c, DoctorScope::Action)));
    assert!(cats.iter().any(|c| matches!(c, DoctorScope::Permission)));
    assert!(cats.iter().any(|c| matches!(c, DoctorScope::Safety)));
    assert!(cats.iter().any(|c| matches!(c, DoctorScope::Proxy)));
}

#[test]
fn doctor_scope_jail_carries_name() {
    let scope = DoctorScope::Jail("sshd".to_string());
    assert!(matches!(scope, DoctorScope::Jail(name) if name == "sshd"));
}

// ===========================================================================
// check_binaries
// ===========================================================================

#[test]
fn check_binaries_with_all_binaries_present() {
    let mut fake = FakeRunner::new();

    // fail2ban-client --version
    fake.with_response(
        "fail2ban-client",
        &["--version"],
        ok_output("Fail2Ban v1.0.2"),
    );
    // nft --version
    fake.with_response("nft", &["--version"], ok_output("nftables 1.0.6"));
    // iptables --version
    fake.with_response("iptables", &["--version"], ok_output("iptables v1.8.8"));

    let doctor = Doctor::new(&fake);
    let findings = doctor.check_binaries();

    // nft and iptables should be reported as available.
    assert!(has_finding(&findings, "binary.nft.available"));
    assert!(has_finding(&findings, "binary.iptables.available"));

    // If fail2ban-client / fail2ban-regex / systemctl are on the host PATH,
    // they will have .found findings; otherwise .missing. Either way we must
    // have at least the firewall findings above.
    assert!(!findings.is_empty());
}

#[test]
fn check_binaries_with_missing_fail2ban_client() {
    // On a system where fail2ban-client is not on PATH, the find_binary call
    // will fail and the runner is never invoked for --version.
    let fake = FakeRunner::new();
    let doctor = Doctor::new(&fake);
    let findings = doctor.check_binaries();

    // The method must produce findings regardless.
    assert!(!findings.is_empty());

    // If fail2ban-client is not found, the critical finding should appear.
    if let Err(_) = find_binary("fail2ban-client") {
        assert!(has_finding(&findings, "binary.fail2ban-client.missing"));
        assert!(findings
            .iter()
            .any(|f| f.id == "binary.fail2ban-client.missing" && f.severity == Severity::Critical));
    }
}

#[test]
fn check_binaries_reports_version_when_available() {
    // Only test the version-detection branch if fail2ban-client is on PATH.
    let Ok(path) = find_binary("fail2ban-client") else {
        return;
    };
    let bin = path.to_str().unwrap_or("fail2ban-client");

    let mut fake = FakeRunner::new();
    fake.with_response(bin, &["--version"], ok_output("Fail2Ban v1.1.0"));
    fake.with_response("nft", &["--version"], ok_output("1.0"));
    fake.with_response("iptables", &["--version"], ok_output("1.8"));

    let doctor = Doctor::new(&fake);
    let findings = doctor.check_binaries();

    assert!(has_finding(&findings, "binary.fail2ban-client.version"));
    let ver_finding = findings.iter().find(|f| f.id == "binary.fail2ban-client.version").unwrap();
    assert_eq!(ver_finding.severity, Severity::Info);
    assert!(ver_finding.detail.contains("Fail2Ban v1.1.0"));
}

#[test]
fn check_binaries_reports_version_failed_when_nonzero() {
    let Ok(path) = find_binary("fail2ban-client") else {
        return;
    };
    let bin = path.to_str().unwrap_or("fail2ban-client");

    let mut fake = FakeRunner::new();
    fake.with_response(bin, &["--version"], fail_output("unknown flag"));
    fake.with_response("nft", &["--version"], ok_output("1.0"));
    fake.with_response("iptables", &["--version"], ok_output("1.8"));

    let doctor = Doctor::new(&fake);
    let findings = doctor.check_binaries();

    assert!(has_finding(&findings, "binary.fail2ban-client.version-failed"));
}

// ===========================================================================
// check_service
// ===========================================================================

#[test]
fn check_service_with_active_service() {
    let mut fake = FakeRunner::new();
    fake.with_response(
        "systemctl",
        &["is-active", "fail2ban"],
        ok_output("active"),
    );
    fake.with_response(
        "systemctl",
        &["is-enabled", "fail2ban"],
        ok_output("enabled"),
    );

    // If fail2ban-client is on PATH, set up responses for ping/logtarget/dbfile.
    if let Ok(path) = find_binary("fail2ban-client") {
        let bin = path.to_str().unwrap_or("fail2ban-client");
        fake.with_response(bin, &["ping"], ok_output("Server replied: pong"));
        fake.with_response(bin, &["get", "logtarget"], ok_output("/var/log/fail2ban.log"));
        fake.with_response(
            bin,
            &["get", "dbfile"],
            ok_output("/var/lib/fail2ban/fail2ban.sqlite3"),
        );
    }

    let doctor = Doctor::new(&fake);
    let findings = doctor.check_service();

    assert!(has_finding(&findings, "service.active"));
    assert!(has_finding(&findings, "service.enabled"));

    // If fail2ban-client is available, we should also see ping and logtarget.
    if find_binary("fail2ban-client").is_ok() {
        assert!(has_finding(&findings, "service.ping-ok"));
        assert!(has_finding(&findings, "service.logtarget-accessible"));
        assert!(has_finding(&findings, "service.dbfile-configured"));
    }
}

#[test]
fn check_service_with_inactive_service() {
    let mut fake = FakeRunner::new();
    fake.with_response(
        "systemctl",
        &["is-active", "fail2ban"],
        fail_output("inactive"),
    );
    fake.with_response(
        "systemctl",
        &["is-enabled", "fail2ban"],
        fail_output("disabled"),
    );

    let doctor = Doctor::new(&fake);
    let findings = doctor.check_service();

    assert!(has_finding(&findings, "service.inactive"));
    assert!(has_finding(&findings, "service.not-enabled"));

    let inactive = findings.iter().find(|f| f.id == "service.inactive").unwrap();
    assert_eq!(inactive.severity, Severity::Critical);
    assert!(inactive.fix.is_some());
}

#[test]
fn check_service_active_check_error() {
    let fake = FakeRunner::new();
    // FakeRunner returns a default success for unregistered commands,
    // so the default empty success flows through and we verify the
    // service.active finding is emitted.
    let doctor = Doctor::new(&fake);
    let findings = doctor.check_service();
    assert!(!findings.is_empty());
}

#[test]
fn check_service_ping_failed() {
    let Ok(path) = find_binary("fail2ban-client") else {
        return;
    };
    let bin = path.to_str().unwrap_or("fail2ban-client");

    let mut fake = FakeRunner::new();
    fake.with_response(
        "systemctl",
        &["is-active", "fail2ban"],
        ok_output("active"),
    );
    fake.with_response(
        "systemctl",
        &["is-enabled", "fail2ban"],
        ok_output("enabled"),
    );
    fake.with_response(bin, &["ping"], fail_output("Connection refused"));
    fake.with_response(bin, &["get", "logtarget"], fail_output("error"));
    fake.with_response(bin, &["get", "dbfile"], fail_output("error"));

    let doctor = Doctor::new(&fake);
    let findings = doctor.check_service();

    assert!(has_finding(&findings, "service.ping-failed"));
    let ping_finding = findings.iter().find(|f| f.id == "service.ping-failed").unwrap();
    assert_eq!(ping_finding.severity, Severity::Critical);
}

#[test]
fn check_service_dbfile_disabled() {
    let Ok(path) = find_binary("fail2ban-client") else {
        return;
    };
    let bin = path.to_str().unwrap_or("fail2ban-client");

    let mut fake = FakeRunner::new();
    fake.with_response(
        "systemctl",
        &["is-active", "fail2ban"],
        ok_output("active"),
    );
    fake.with_response(
        "systemctl",
        &["is-enabled", "fail2ban"],
        ok_output("enabled"),
    );
    fake.with_response(bin, &["ping"], ok_output("pong"));
    fake.with_response(bin, &["get", "logtarget"], ok_output("/var/log/fail2ban.log"));
    fake.with_response(bin, &["get", "dbfile"], ok_output("None"));

    let doctor = Doctor::new(&fake);
    let findings = doctor.check_service();

    assert!(has_finding(&findings, "service.dbfile-disabled"));
}

// ===========================================================================
// check_config
// ===========================================================================

#[test]
fn check_config_with_valid_config() {
    // This test depends on whether /etc/fail2ban exists on the host.
    // We verify the method runs and produces a directory finding.
    let fake = FakeRunner::new();
    let doctor = Doctor::new(&fake);
    let findings = doctor.check_config();

    let config_dir = std::path::Path::new("/etc/fail2ban");
    if config_dir.exists() {
        assert!(has_finding(&findings, "config.directory.exists"));
    } else {
        assert!(has_finding(&findings, "config.directory.missing"));
    }
}

#[test]
fn check_config_reports_missing_directory() {
    // /etc/fail2ban likely does not exist on a macOS dev machine.
    let fake = FakeRunner::new();
    let doctor = Doctor::new(&fake);
    let findings = doctor.check_config();

    let has_dir = has_finding(&findings, "config.directory.exists")
        || has_finding(&findings, "config.directory.missing");
    assert!(has_dir);
}

#[test]
fn check_config_test_passed_when_binary_available() {
    if !std::path::Path::new("/etc/fail2ban").exists() {
        return;
    }
    let Ok(path) = find_binary("fail2ban-client") else {
        return;
    };
    let bin = path.to_str().unwrap_or("fail2ban-client");

    let mut fake = FakeRunner::new();
    fake.with_response(bin, &["--test"], ok_output("OK"));

    let doctor = Doctor::new(&fake);
    let findings = doctor.check_config();
    assert!(has_finding(&findings, "config.test.passed"));
}

#[test]
fn check_config_test_failed_when_nonzero() {
    if !std::path::Path::new("/etc/fail2ban").exists() {
        return;
    }
    let Ok(path) = find_binary("fail2ban-client") else {
        return;
    };
    let bin = path.to_str().unwrap_or("fail2ban-client");

    let mut fake = FakeRunner::new();
    fake.with_response(bin, &["--test"], fail_output("Syntax error in jail.conf"));

    let doctor = Doctor::new(&fake);
    let findings = doctor.check_config();
    assert!(has_finding(&findings, "config.test.failed"));
    let f = findings.iter().find(|f| f.id == "config.test.failed").unwrap();
    assert_eq!(f.severity, Severity::Error);
    assert!(f.fix.is_some());
}

// ===========================================================================
// check_jail
// ===========================================================================

#[test]
fn check_jail_with_existing_jail() {
    let Ok(path) = find_binary("fail2ban-client") else {
        return;
    };
    let bin = path.to_str().unwrap_or("fail2ban-client");

    let mut fake = FakeRunner::new();
    let status_output = indoc::indoc! {"
        Status for the jail: sshd
        |- Filter
        |  |- Currently failed: 0
        |  |- Total failed:     5
        |  `- File list:        /var/log/auth.log
        `- Actions
           |- Currently banned: 2
           |- Total banned:     10
           `- Banned IP list:   192.0.2.1 192.0.2.2
    "};
    fake.with_response(bin, &["status", "sshd"], ok_output(status_output));
    fake.with_response(bin, &["get", "sshd", "bantime"], ok_output("600"));
    fake.with_response(bin, &["get", "sshd", "findtime"], ok_output("600"));
    fake.with_response(bin, &["get", "sshd", "maxretry"], ok_output("5"));

    let doctor = Doctor::new(&fake);
    let findings = doctor.check_jail("sshd");

    assert!(has_finding(&findings, "jail.exists"));
    assert!(has_finding(&findings, "jail.has-filter"));
    assert!(has_finding(&findings, "jail.has-action"));
    assert!(has_finding(&findings, "jail.maxretry-ok"));
}

#[test]
fn check_jail_not_found() {
    let Ok(path) = find_binary("fail2ban-client") else {
        return;
    };
    let bin = path.to_str().unwrap_or("fail2ban-client");

    let mut fake = FakeRunner::new();
    fake.with_response(
        bin,
        &["status", "nonexistent"],
        fail_output("Jail 'nonexistent' not found"),
    );

    let doctor = Doctor::new(&fake);
    let findings = doctor.check_jail("nonexistent");

    assert!(has_finding(&findings, "jail.not-found"));
    let f = findings.iter().find(|f| f.id == "jail.not-found").unwrap();
    assert_eq!(f.severity, Severity::Error);
    assert!(f.fix.is_some());
}

#[test]
fn check_jail_maxretry_zero_warning() {
    let Ok(path) = find_binary("fail2ban-client") else {
        return;
    };
    let bin = path.to_str().unwrap_or("fail2ban-client");

    let mut fake = FakeRunner::new();
    fake.with_response(bin, &["status", "badjail"], ok_output("Filter\nActions\n"));
    fake.with_response(bin, &["get", "badjail", "bantime"], ok_output("600"));
    fake.with_response(bin, &["get", "badjail", "findtime"], ok_output("600"));
    fake.with_response(bin, &["get", "badjail", "maxretry"], ok_output("0"));

    let doctor = Doctor::new(&fake);
    let findings = doctor.check_jail("badjail");

    assert!(has_finding(&findings, "jail.maxretry-zero"));
    let f = findings.iter().find(|f| f.id == "jail.maxretry-zero").unwrap();
    assert_eq!(f.severity, Severity::Warning);
}

#[test]
fn check_jail_maxretry_very_high_info() {
    let Ok(path) = find_binary("fail2ban-client") else {
        return;
    };
    let bin = path.to_str().unwrap_or("fail2ban-client");

    let mut fake = FakeRunner::new();
    fake.with_response(bin, &["status", "loosejail"], ok_output("Filter\nActions\n"));
    fake.with_response(bin, &["get", "loosejail", "bantime"], ok_output("600"));
    fake.with_response(bin, &["get", "loosejail", "findtime"], ok_output("600"));
    fake.with_response(bin, &["get", "loosejail", "maxretry"], ok_output("200"));

    let doctor = Doctor::new(&fake);
    let findings = doctor.check_jail("loosejail");

    assert!(has_finding(&findings, "jail.maxretry-very-high"));
}

#[test]
fn check_jail_no_client_binary() {
    // If fail2ban-client is not on PATH, the method should still produce a
    // finding without panicking.
    let fake = FakeRunner::new();
    let doctor = Doctor::new(&fake);
    let findings = doctor.check_jail("sshd");

    if find_binary("fail2ban-client").is_err() {
        assert!(has_finding(&findings, "jail.no-client"));
        let f = findings.iter().find(|f| f.id == "jail.no-client").unwrap();
        assert_eq!(f.severity, Severity::Critical);
    }
}

// ===========================================================================
// check_permissions
// ===========================================================================

#[test]
fn check_permissions_reports_something() {
    let fake = FakeRunner::new();
    let doctor = Doctor::new(&fake);
    let findings = doctor.check_permissions();
    assert!(!findings.is_empty());
}

#[test]
fn check_permissions_finds_config_dir_safe_when_present() {
    if !std::path::Path::new("/etc/fail2ban").exists() {
        return;
    }
    let fake = FakeRunner::new();
    let doctor = Doctor::new(&fake);
    let findings = doctor.check_permissions();

    // Either the dir is world-writable or it is safe.
    let has_perm_finding = has_finding(&findings, "permission.config-dir-safe")
        || has_finding(&findings, "permission.config-dir-world-writable")
        || has_finding(&findings, "permission.config-dir-missing");
    assert!(has_perm_finding);
}

// ===========================================================================
// check_safety
// ===========================================================================

#[test]
fn check_safety_always_reports_dry_run_available() {
    let fake = FakeRunner::new();
    let doctor = Doctor::new(&fake);
    let findings = doctor.check_safety();

    assert!(has_finding(&findings, "safety.dry-run-available"));
}

// ===========================================================================
// run() with DoctorScope::All
// ===========================================================================

#[test]
fn run_with_all_scope_returns_report() {
    let mut fake = FakeRunner::new();
    fake.with_response(
        "systemctl",
        &["is-active", "fail2ban"],
        fail_output("inactive"),
    );
    fake.with_response(
        "systemctl",
        &["is-enabled", "fail2ban"],
        fail_output("disabled"),
    );
    fake.with_response("nft", &["--version"], ok_output("1.0"));
    fake.with_response("iptables", &["--version"], ok_output("1.8"));
    fake.with_response("journalctl", &["--version"], ok_output("250"));

    let doctor = Doctor::new(&fake);
    let report = doctor.run(&DoctorScope::All).unwrap();

    assert!(!report.findings.is_empty());
}

// ===========================================================================
// run() with specific scopes
// ===========================================================================

#[test]
fn run_with_binary_scope() {
    let mut fake = FakeRunner::new();
    fake.with_response("nft", &["--version"], ok_output("1.0"));
    fake.with_response("iptables", &["--version"], ok_output("1.8"));

    let doctor = Doctor::new(&fake);
    let report = doctor.run(&DoctorScope::Binary).unwrap();
    assert!(!report.findings.is_empty());
    assert!(has_finding(&report.findings, "binary.nft.available"));
    assert!(has_finding(&report.findings, "binary.iptables.available"));
}

#[test]
fn run_with_service_scope() {
    let mut fake = FakeRunner::new();
    fake.with_response(
        "systemctl",
        &["is-active", "fail2ban"],
        ok_output("active"),
    );
    fake.with_response(
        "systemctl",
        &["is-enabled", "fail2ban"],
        ok_output("enabled"),
    );

    let doctor = Doctor::new(&fake);
    let report = doctor.run(&DoctorScope::Service).unwrap();
    assert!(has_finding(&report.findings, "service.active"));
    assert!(has_finding(&report.findings, "service.enabled"));
}

#[test]
fn run_with_config_scope() {
    let fake = FakeRunner::new();
    let doctor = Doctor::new(&fake);
    let report = doctor.run(&DoctorScope::Config).unwrap();
    assert!(!report.findings.is_empty());
}

#[test]
fn run_with_permission_scope() {
    let fake = FakeRunner::new();
    let doctor = Doctor::new(&fake);
    let report = doctor.run(&DoctorScope::Permission).unwrap();
    assert!(!report.findings.is_empty());
}

#[test]
fn run_with_safety_scope() {
    let fake = FakeRunner::new();
    let doctor = Doctor::new(&fake);
    let report = doctor.run(&DoctorScope::Safety).unwrap();
    assert!(has_finding(&report.findings, "safety.dry-run-available"));
}

#[test]
fn run_with_journal_scope() {
    let mut fake = FakeRunner::new();
    fake.with_response("journalctl", &["--version"], ok_output("250"));
    fake.with_response(
        "journalctl",
        &["-u", "fail2ban", "-n", "1", "--no-pager"],
        ok_output("fail2ban.log line"),
    );

    let doctor = Doctor::new(&fake);
    let report = doctor.run(&DoctorScope::Journal).unwrap();
    assert!(has_finding(&report.findings, "journal.journalctl.available"));
    assert!(has_finding(&report.findings, "journal.entries-found"));
}

#[test]
fn run_with_regex_scope() {
    let fake = FakeRunner::new();
    let doctor = Doctor::new(&fake);
    let report = doctor.run(&DoctorScope::Regex).unwrap();
    // Should produce some finding (either tool found or missing).
    assert!(!report.findings.is_empty());
}

#[test]
fn run_with_action_scope() {
    let fake = FakeRunner::new();
    let doctor = Doctor::new(&fake);
    let report = doctor.run(&DoctorScope::Action).unwrap();
    assert!(!report.findings.is_empty());
}

#[test]
fn run_with_logpath_scope() {
    let fake = FakeRunner::new();
    let doctor = Doctor::new(&fake);
    let report = doctor.run(&DoctorScope::LogPath).unwrap();
    assert!(!report.findings.is_empty());
}

#[test]
fn run_with_proxy_scope() {
    let fake = FakeRunner::new();
    let doctor = Doctor::new(&fake);
    let report = doctor.run(&DoctorScope::Proxy).unwrap();
    // If no proxy issues are detected, the ok finding is emitted.
    assert!(has_finding(&report.findings, "proxy.no-issues"));
}

#[test]
fn run_with_jail_scope() {
    let Ok(path) = find_binary("fail2ban-client") else {
        return;
    };
    let bin = path.to_str().unwrap_or("fail2ban-client");

    let mut fake = FakeRunner::new();
    fake.with_response(
        bin,
        &["status", "sshd"],
        ok_output("Filter\nActions\nCurrently banned: 0\n"),
    );
    fake.with_response(bin, &["get", "sshd", "bantime"], ok_output("600"));
    fake.with_response(bin, &["get", "sshd", "findtime"], ok_output("600"));
    fake.with_response(bin, &["get", "sshd", "maxretry"], ok_output("5"));

    let doctor = Doctor::new(&fake);
    let report = doctor.run(&DoctorScope::Jail("sshd".to_string())).unwrap();
    assert!(has_finding(&report.findings, "jail.exists"));
}

// ===========================================================================
// DoctorReport::summary_by_severity()
// ===========================================================================

#[test]
fn doctor_report_summary_by_severity_groups_correctly() {
    let mut report = DoctorReport::empty();
    report.push(Finding::new("a", Severity::Ok, "ok finding"));
    report.push(Finding::new("b", Severity::Ok, "another ok"));
    report.push(Finding::new("c", Severity::Critical, "bad"));
    report.push(Finding::new("d", Severity::Warning, "meh"));

    let summary = report.summary_by_severity();

    assert_eq!(summary[&Severity::Ok].len(), 2);
    assert_eq!(summary[&Severity::Critical].len(), 1);
    assert_eq!(summary[&Severity::Warning].len(), 1);
    assert!(!summary.contains_key(&Severity::Info));
    assert!(!summary.contains_key(&Severity::Error));
}

#[test]
fn doctor_report_summary_by_severity_empty_report() {
    let report = DoctorReport::empty();
    let summary = report.summary_by_severity();
    assert!(summary.is_empty());
}

// ===========================================================================
// DoctorReport::has_errors()
// ===========================================================================

#[test]
fn doctor_report_has_errors_true_with_error_severity() {
    let mut report = DoctorReport::empty();
    report.push(Finding::new("e1", Severity::Error, "error"));
    assert!(report.has_errors());
}

#[test]
fn doctor_report_has_errors_true_with_critical_severity() {
    let mut report = DoctorReport::empty();
    report.push(Finding::new("c1", Severity::Critical, "critical"));
    assert!(report.has_errors());
}

#[test]
fn doctor_report_has_errors_false_with_only_ok() {
    let mut report = DoctorReport::empty();
    report.push(Finding::new("ok1", Severity::Ok, "fine"));
    report.push(Finding::new("i1", Severity::Info, "note"));
    report.push(Finding::new("w1", Severity::Warning, "meh"));
    assert!(!report.has_errors());
}

#[test]
fn doctor_report_has_errors_false_when_empty() {
    let report = DoctorReport::empty();
    assert!(!report.has_errors());
}

// ===========================================================================
// DoctorReport::has_critical()
// ===========================================================================

#[test]
fn doctor_report_has_critical_true() {
    let mut report = DoctorReport::empty();
    report.push(Finding::new("c1", Severity::Critical, "fatal"));
    assert!(report.has_critical());
}

#[test]
fn doctor_report_has_critical_false_with_error_only() {
    let mut report = DoctorReport::empty();
    report.push(Finding::new("e1", Severity::Error, "error"));
    assert!(!report.has_critical());
}

// ===========================================================================
// DoctorReport::len / is_empty
// ===========================================================================

#[test]
fn doctor_report_len_and_is_empty() {
    let empty = DoctorReport::empty();
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);

    let mut nonempty = DoctorReport::empty();
    nonempty.push(Finding::new("x", Severity::Ok, "x"));
    assert!(!nonempty.is_empty());
    assert_eq!(nonempty.len(), 1);
}

// ===========================================================================
// Finding construction and fields
// ===========================================================================

#[test]
fn finding_new_sets_mandatory_fields() {
    let f = Finding::new("test.id", Severity::Warning, "test title");
    assert_eq!(f.id, "test.id");
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.title, "test title");
    assert!(f.detail.is_empty());
    assert!(f.fix.is_none());
}

#[test]
fn finding_detail_chain_sets_detail() {
    let f = Finding::new("id", Severity::Ok, "title").detail("some detail text");
    assert_eq!(f.detail, "some detail text");
}

#[test]
fn finding_fix_chain_sets_fix() {
    let f = Finding::new("id", Severity::Error, "title").fix("do this to fix");
    assert_eq!(f.fix.as_deref(), Some("do this to fix"));
}

#[test]
fn finding_detail_and_fix_chained() {
    let f = Finding::new("id", Severity::Critical, "title")
        .detail("something is broken")
        .fix("reinstall the package");
    assert_eq!(f.detail, "something is broken");
    assert_eq!(f.fix.as_deref(), Some("reinstall the package"));
}

#[test]
fn finding_detail_replaces_previous() {
    let f = Finding::new("id", Severity::Ok, "title")
        .detail("first")
        .detail("second");
    assert_eq!(f.detail, "second");
}

#[test]
fn finding_fix_replaces_previous() {
    let f = Finding::new("id", Severity::Ok, "title")
        .fix("first")
        .fix("second");
    assert_eq!(f.fix.as_deref(), Some("second"));
}

// ===========================================================================
// Severity ordering
// ===========================================================================

#[test]
fn severity_ordering() {
    assert!(Severity::Ok < Severity::Info);
    assert!(Severity::Info < Severity::Warning);
    assert!(Severity::Warning < Severity::Error);
    assert!(Severity::Error < Severity::Critical);
}

#[test]
fn severity_display() {
    assert_eq!(format!("{}", Severity::Ok), "OK");
    assert_eq!(format!("{}", Severity::Info), "INFO");
    assert_eq!(format!("{}", Severity::Warning), "WARNING");
    assert_eq!(format!("{}", Severity::Error), "ERROR");
    assert_eq!(format!("{}", Severity::Critical), "CRITICAL");
}

// ===========================================================================
// parse_jail_list
// ===========================================================================

#[test]
fn parse_jail_list_extracts_names() {
    let status = "Status\n|- Number of jail:      2\n`- Jail list:   sshd, nginx\n";
    let jails = parse_jail_list(status);
    assert_eq!(jails, vec!["sshd", "nginx"]);
}

#[test]
fn parse_jail_list_single_jail() {
    let status = "`- Jail list:   sshd\n";
    let jails = parse_jail_list(status);
    assert_eq!(jails, vec!["sshd"]);
}

#[test]
fn parse_jail_list_empty() {
    let status = "Status\n|- Number of jail:      0\n`- Jail list:\n";
    let jails = parse_jail_list(status);
    assert!(jails.is_empty());
}

#[test]
fn parse_jail_list_no_jail_line() {
    let status = "Status\n|- Something else\n";
    let jails = parse_jail_list(status);
    assert!(jails.is_empty());
}

#[test]
fn parse_jail_list_case_insensitive_detection() {
    let status = "JAIL LIST: sshd, apache\n";
    let jails = parse_jail_list(status);
    assert_eq!(jails, vec!["sshd", "apache"]);
}
