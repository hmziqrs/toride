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

fn run_systemctl(runner: &dyn CommandRunner, args: &[&str]) -> Result<CommandResult> {
    let spec = crate::spec::CommandSpec {
        program: "systemctl".into(),
        args: args.iter().map(|s| (*s).to_string()).collect(),
        timeout: Some(std::time::Duration::from_secs(10)),
        requires_root: false,
        force_c_locale: true,
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
    };
    runner.run(&spec)
}

#[cfg(test)]
#[path = "service.test.rs"]
mod tests;
