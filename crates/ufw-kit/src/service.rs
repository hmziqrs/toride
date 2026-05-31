//! System service management for UFW.
//!
//! Uses `systemctl` to check service status.

use crate::command::CommandRunner;
use crate::error::{Error, Result};
use crate::spec::CommandResult;

/// Check if the UFW service is active.
pub fn is_active(runner: &dyn CommandRunner) -> Result<bool> {
    let result = run_systemctl(runner, &["is-active", "ufw"])?;
    Ok(result.stdout.trim() == "active")
}

/// Check if the UFW service is enabled.
pub fn is_enabled(runner: &dyn CommandRunner) -> Result<bool> {
    let result = run_systemctl(runner, &["is-enabled", "ufw"])?;
    Ok(result.stdout.trim() == "enabled")
}

/// Start the UFW service.
pub fn start(runner: &dyn CommandRunner) -> Result<()> {
    let result = run_systemctl_root(runner, &["start", "ufw"])?;
    if result.exit_code != Some(0) {
        return Err(Error::Other(format!(
            "failed to start ufw: {}",
            result.stderr
        )));
    }
    Ok(())
}

/// Stop the UFW service.
pub fn stop(runner: &dyn CommandRunner) -> Result<()> {
    let result = run_systemctl_root(runner, &["stop", "ufw"])?;
    if result.exit_code != Some(0) {
        return Err(Error::Other(format!(
            "failed to stop ufw: {}",
            result.stderr
        )));
    }
    Ok(())
}

/// Restart the UFW service.
pub fn restart(runner: &dyn CommandRunner) -> Result<()> {
    let result = run_systemctl_root(runner, &["restart", "ufw"])?;
    if result.exit_code != Some(0) {
        return Err(Error::Other(format!(
            "failed to restart ufw: {}",
            result.stderr
        )));
    }
    Ok(())
}

/// Reload the UFW service via systemctl.
pub fn reload(runner: &dyn CommandRunner) -> Result<()> {
    let result = run_systemctl_root(runner, &["reload", "ufw"])?;
    if result.exit_code != Some(0) {
        return Err(Error::Other(format!(
            "failed to reload ufw: {}",
            result.stderr
        )));
    }
    Ok(())
}

/// Tail recent UFW journal entries.
///
/// Runs: `journalctl -u ufw --no-pager -n {lines}`
pub fn journal_tail(runner: &dyn CommandRunner, lines: u32) -> Result<String> {
    let result = run_journalctl(runner, lines)?;
    if result.exit_code != Some(0) {
        return Err(Error::Other(format!(
            "failed to read ufw journal: {}",
            result.stderr
        )));
    }
    Ok(result.stdout)
}

fn run_systemctl(runner: &dyn CommandRunner, args: &[&str]) -> Result<CommandResult> {
    let spec = crate::spec::CommandSpec {
        program: "systemctl".into(),
        args: args.iter().map(|s| (*s).to_string()).collect(),
        timeout: Some(std::time::Duration::from_secs(10)),
        requires_root: false,
        force_c_locale: true,
        redact_logs: false,
    };
    runner.run(&spec)
}

fn run_systemctl_root(runner: &dyn CommandRunner, args: &[&str]) -> Result<CommandResult> {
    let spec = crate::spec::CommandSpec {
        program: "systemctl".into(),
        args: args.iter().map(|s| (*s).to_string()).collect(),
        timeout: Some(std::time::Duration::from_secs(30)),
        requires_root: true,
        force_c_locale: true,
        redact_logs: false,
    };
    runner.run(&spec)
}

fn run_journalctl(runner: &dyn CommandRunner, lines: u32) -> Result<CommandResult> {
    let spec = crate::spec::CommandSpec {
        program: "journalctl".into(),
        args: vec![
            "-u".into(),
            "ufw".into(),
            "--no-pager".into(),
            "-n".into(),
            lines.to_string(),
        ],
        timeout: Some(std::time::Duration::from_secs(10)),
        requires_root: false,
        force_c_locale: true,
        redact_logs: false,
    };
    runner.run(&spec)
}

#[cfg(test)]
#[path = "service.test.rs"]
mod tests;
