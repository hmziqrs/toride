//! Example: embedding the `toride_fail2ban` library in an application.
//!
//! Demonstrates how to create a Fail2Ban instance, build a jail spec with
//! an inline filter, ensure the jail is written to disk, test the config,
//! reload the jail, and run a full doctor diagnostic.
//!
//! # Prerequisites
//!
//! This example requires a working Fail2Ban installation (fail2ban-client
//! on `$PATH`, `/etc/fail2ban` present, and appropriate privileges).
//!
//! ```sh
//! cargo run --example embed_myapp
//! ```

use std::process;

use toride_fail2ban::doctor::DoctorScope;
use toride_fail2ban::report::Severity;
use toride_fail2ban::spec::{
    ActionKind, ActionSpec, Backend, FilterSpec, FilterName, JailName, JailSpec, LogPath,
    RegexLine,
};
use toride_fail2ban::spec::{ActionName, DurationSpec};
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

    // -- 2. Build a jail specification for "myapp" --
    println!("Building jail spec for 'myapp'...");
    let spec = JailSpec::builder()
        .name(JailName::from_str("myapp")?)
        .filter(
            FilterSpec::builder()
                .name(FilterName::from_str("myapp-auth")?)
                .failregex(vec![RegexLine::from_str(
                    r"Authentication failure.*<HOST>",
                )?])
                .build(),
        )
        .log_paths(vec![LogPath::from_str("/var/log/myapp/auth.log")?])
        .backend(Backend::Auto)
        .bantime(DurationSpec::from_str("10m")?)
        .findtime(DurationSpec::from_str("10m")?)
        .maxretry(5)
        .actions(vec![
            ActionSpec::builder()
                .name(ActionName::from_str("nftables-multiport")?)
                .kind(ActionKind::Stock)
                .build(),
        ])
        .build();
    println!(
        "  ok: jail={}, filter={}, bantime={}, findtime={}, maxretry={}\n",
        spec.name,
        spec.filter.name,
        spec.bantime,
        spec.findtime,
        spec.maxretry,
    );

    // -- 3. Write the jail config, test, and reload --
    println!("Ensuring jail 'myapp'...");
    let apply = f2b.ensure_jail(spec)?;
    println!("  files written : {:?}", apply.files_written);
    println!("  backups       : {:?}", apply.backup_paths);
    println!("  test passed   : {}", apply.test_passed);
    println!("  reload result : {:?}\n", apply.reload_result);

    if !apply.findings.is_empty() {
        println!("  apply findings:");
        for finding in &apply.findings {
            println!("    [{}] {}", finding.severity, finding.title);
        }
        println!();
    }

    // -- 4. Validate the full Fail2Ban configuration --
    println!("Testing Fail2Ban configuration...");
    f2b.test_config()?;
    println!("  ok: fail2ban-client --test passed\n");

    // -- 5. Reload just the "myapp" jail --
    println!("Reloading jail 'myapp'...");
    f2b.reload_jail("myapp")?;
    println!("  ok: jail reloaded\n");

    // -- 6. Run the doctor across all diagnostic categories --
    println!("Running doctor (all categories)...");
    let report = f2b.doctor(DoctorScope::All)?;
    println!("  findings: {}", report.len());

    if report.has_critical() {
        println!("\n  CRITICAL issues detected:");
    }

    let by_severity = report.summary_by_severity();
    for (level, findings) in &by_severity {
        println!("\n  [{level}] ({} finding(s))", findings.len());
        for f in findings {
            println!("    - {} ({})", f.title, f.id);
            if !f.detail.is_empty() {
                println!("      {}", f.detail);
            }
            if let Some(fix) = &f.fix {
                println!("      Fix: {fix}");
            }
        }
    }

    if report.has_errors() {
        println!("\nDoctor found errors. Review the findings above before relying on this Fail2Ban setup.");
    } else {
        println!("\nDoctor completed with no blocking issues.");
    }

    Ok(())
}
