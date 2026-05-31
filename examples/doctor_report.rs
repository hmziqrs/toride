//! Example: running a doctor report with the `toride_fail2ban` library.
//!
//! Demonstrates how to create a Fail2Ban instance, run the full doctor
//! diagnostic, and present findings grouped by severity with human-friendly
//! indicators and fix suggestions.
//!
//! # Prerequisites
//!
//! This example requires a working Fail2Ban installation (fail2ban-client
//! on `$PATH`, `/etc/fail2ban` present, and appropriate privileges).
//!
//! ```sh
//! cargo run --example doctor_report
//! ```

use std::process;

use toride_fail2ban::doctor::DoctorScope;
use toride_fail2ban::report::Severity;
use toride_fail2ban::Fail2Ban;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // -- 1. Create a Fail2Ban instance bound to the system installation --
    println!("Connecting to system Fail2Ban...");
    let f2b = Fail2Ban::system()?;
    println!("  ok: connected to /etc/fail2ban\n");

    // -- 2. Run the doctor across all diagnostic categories --
    println!("Running doctor (all categories)...");
    let report = f2b.doctor(DoctorScope::All)?;
    println!("  total findings: {}\n", report.len());

    if report.is_empty() {
        println!("  All checks passed -- no findings.\n");
        return Ok(());
    }

    // -- 3. Iterate over findings grouped by severity --
    let by_severity = report.summary_by_severity();
    for (level, findings) in &by_severity {
        // -- 4. Print severity indicator and count --
        let icon = severity_icon(*level);
        println!("{icon} {level} ({} finding(s))", findings.len());

        for f in findings {
            println!("  - {}", f.title);
            if !f.detail.is_empty() {
                println!("    {}", f.detail);
            }
            // -- 5. Show fix suggestions --
            if let Some(fix) = &f.fix {
                println!("    Fix: {fix}");
            }
        }
        println!();
    }

    // -- 6. Exit with code 1 if critical issues found --
    if report.has_critical() {
        eprintln!("Critical issues detected. Address them before relying on this Fail2Ban setup.");
        process::exit(1);
    }

    if report.has_errors() {
        eprintln!("Errors found. Review the findings above before relying on this Fail2Ban setup.");
    }

    Ok(())
}

/// Returns an emoji indicator for the given severity level.
fn severity_icon(severity: Severity) -> &'static str {
    match severity {
        Severity::Ok => "\u{2705}",       // Ok
        Severity::Info => "\u{2139}\u{fe0f}", // Info
        Severity::Warning => "\u{26a0}\u{fe0f}", // Warning
        Severity::Error => "\u{274c}",    // Error
        Severity::Critical => "\u{1f534}", // Critical
    }
}
