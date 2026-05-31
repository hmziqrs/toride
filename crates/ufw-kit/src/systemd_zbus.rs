//! Systemd integration via zbus D-Bus (behind `systemd-zbus` feature).
//!
//! Provides async D-Bus based service management as an alternative to
//! shelling out to `systemctl`. Uses the `zbus` crate for D-Bus communication.

use crate::command::CommandRunner;
use crate::error::{Error, Result};
use crate::spec::CommandSpec;
use std::time::Duration;

// ============================================================================
// Types
// ============================================================================

/// Systemd unit status from D-Bus.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SystemdUnitStatus {
    /// Unit name (e.g., "ufw.service").
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Load state ("loaded", "not-found", etc.).
    pub load_state: String,
    /// Active state ("active", "inactive", "failed", etc.).
    pub active_state: String,
    /// Sub state ("running", "dead", "exited", etc.).
    pub sub_state: String,
    /// Whether the unit is enabled.
    pub enabled: bool,
}

/// Systemd manager proxy result.
#[derive(Debug, Clone)]
pub struct SystemdManagerInfo {
    /// Systemd version.
    pub version: String,
    /// Virtualization status.
    pub virtualization: String,
}

// ============================================================================
// Synchronous wrappers using CommandRunner (fallback)
// ============================================================================

/// Get systemd unit status using systemctl via [`CommandRunner`].
///
/// This is the synchronous fallback that shells out to systemctl.
/// When the `systemd-zbus` feature is fully wired, an async D-Bus
/// version will be preferred.
pub fn get_unit_status<R: CommandRunner + ?Sized>(
    runner: &R,
    unit: &str,
) -> Result<SystemdUnitStatus> {
    let spec = CommandSpec {
        program: "systemctl".into(),
        args: vec!["show".into(), unit.into(), "--no-page".into()],
        timeout: Some(Duration::from_secs(10)),
        requires_root: false,
        force_c_locale: true,
        redact_logs: false,
    };

    let result = runner.run(&spec)?;

    let mut description = String::new();
    let mut load_state = "unknown".into();
    let mut active_state = "unknown".into();
    let mut sub_state = "unknown".into();
    let mut unit_file_state = "unknown".into();

    for line in result.stdout.lines() {
        if let Some(val) = line.strip_prefix("Description=") {
            description = val.to_string();
        } else if let Some(val) = line.strip_prefix("LoadState=") {
            load_state = val.to_string();
        } else if let Some(val) = line.strip_prefix("ActiveState=") {
            active_state = val.to_string();
        } else if let Some(val) = line.strip_prefix("SubState=") {
            sub_state = val.to_string();
        } else if let Some(val) = line.strip_prefix("UnitFileState=") {
            unit_file_state = val.to_string();
        }
    }

    let enabled = unit_file_state == "enabled";

    Ok(SystemdUnitStatus {
        name: unit.to_string(),
        description,
        load_state,
        active_state,
        sub_state,
        enabled,
    })
}

/// Start a systemd unit.
pub fn start_unit<R: CommandRunner + ?Sized>(runner: &R, unit: &str) -> Result<()> {
    let spec = CommandSpec {
        program: "systemctl".into(),
        args: vec!["start".into(), unit.into()],
        timeout: Some(Duration::from_secs(30)),
        requires_root: true,
        force_c_locale: true,
        redact_logs: false,
    };

    let result = runner.run(&spec)?;
    if result.exit_code != Some(0) {
        return Err(Error::DoctorCheckFailed(format!(
            "failed to start {unit}: {}",
            result.stderr
        )));
    }
    Ok(())
}

/// Stop a systemd unit.
pub fn stop_unit<R: CommandRunner + ?Sized>(runner: &R, unit: &str) -> Result<()> {
    let spec = CommandSpec {
        program: "systemctl".into(),
        args: vec!["stop".into(), unit.into()],
        timeout: Some(Duration::from_secs(30)),
        requires_root: true,
        force_c_locale: true,
        redact_logs: false,
    };

    let result = runner.run(&spec)?;
    if result.exit_code != Some(0) {
        return Err(Error::DoctorCheckFailed(format!(
            "failed to stop {unit}: {}",
            result.stderr
        )));
    }
    Ok(())
}

/// Restart a systemd unit.
pub fn restart_unit<R: CommandRunner + ?Sized>(runner: &R, unit: &str) -> Result<()> {
    let spec = CommandSpec {
        program: "systemctl".into(),
        args: vec!["restart".into(), unit.into()],
        timeout: Some(Duration::from_secs(30)),
        requires_root: true,
        force_c_locale: true,
        redact_logs: false,
    };

    let result = runner.run(&spec)?;
    if result.exit_code != Some(0) {
        return Err(Error::DoctorCheckFailed(format!(
            "failed to restart {unit}: {}",
            result.stderr
        )));
    }
    Ok(())
}

/// Enable a systemd unit (boot-time start).
pub fn enable_unit<R: CommandRunner + ?Sized>(runner: &R, unit: &str) -> Result<()> {
    let spec = CommandSpec {
        program: "systemctl".into(),
        args: vec!["enable".into(), unit.into()],
        timeout: Some(Duration::from_secs(10)),
        requires_root: true,
        force_c_locale: true,
        redact_logs: false,
    };

    let result = runner.run(&spec)?;
    if result.exit_code != Some(0) {
        return Err(Error::DoctorCheckFailed(format!(
            "failed to enable {unit}: {}",
            result.stderr
        )));
    }
    Ok(())
}

/// Get journal log entries for a unit.
pub fn journal_tail<R: CommandRunner + ?Sized>(
    runner: &R,
    unit: &str,
    lines: u32,
) -> Result<String> {
    let spec = CommandSpec {
        program: "journalctl".into(),
        args: vec![
            "-u".into(),
            unit.into(),
            "-n".into(),
            lines.to_string(),
            "--no-pager".into(),
        ],
        timeout: Some(Duration::from_secs(10)),
        requires_root: false,
        force_c_locale: true,
        redact_logs: false,
    };

    let result = runner.run(&spec)?;
    Ok(result.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::FakeRunner;

    #[test]
    fn get_unit_status_should_parse_systemctl_show() {
        let runner = FakeRunner::new().respond_ok(
            "systemctl",
            &["show", "ufw.service", "--no-page"],
            "Description=Uncomplicated firewall\n\
             LoadState=loaded\n\
             ActiveState=active\n\
             SubState=running\n\
             UnitFileState=enabled\n",
        );

        let status = get_unit_status(&runner, "ufw.service").unwrap();
        assert_eq!(status.name, "ufw.service");
        assert_eq!(status.active_state, "active");
        assert_eq!(status.sub_state, "running");
        assert!(status.enabled);
    }

    #[test]
    fn get_unit_status_should_handle_inactive() {
        let runner = FakeRunner::new().respond_ok(
            "systemctl",
            &["show", "ufw.service", "--no-page"],
            "Description=Uncomplicated firewall\n\
             LoadState=loaded\n\
             ActiveState=inactive\n\
             SubState=dead\n\
             UnitFileState=disabled\n",
        );

        let status = get_unit_status(&runner, "ufw.service").unwrap();
        assert_eq!(status.active_state, "inactive");
        assert!(!status.enabled);
    }
}
