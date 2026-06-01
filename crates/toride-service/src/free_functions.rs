//! Standalone convenience functions using a default `DuctRunner`.
//!
//! These functions provide quick access to common systemd service queries
//! without requiring explicit construction of a [`ServiceManager`](crate::ServiceManager).
//!
//! Each function creates a temporary `DuctRunner`, so they are intended for
//! one-off calls. For repeated operations, prefer constructing a
//! [`ServiceManager`](crate::ServiceManager) once and reusing it.
//!
//! Only available when the `client` feature is enabled.

#[cfg(feature = "client")]
use crate::{Error, Result};
#[cfg(feature = "client")]
use toride_runner::{CommandSpec, Runner};

/// Check whether the given service is currently active (running).
///
/// # Errors
///
/// Returns [`Error::CommandFailed`] if `systemctl` cannot be executed.
#[cfg(feature = "client")]
pub fn is_active(service: &str) -> Result<bool> {
    let runner = toride_runner::DuctRunner;
    let spec = CommandSpec::new("systemctl").arg("is-active").arg(service);
    let output = runner.run(&spec)?;
    Ok(output.success)
}

/// Start the given service unit.
///
/// # Errors
///
/// Returns [`Error::CommandFailed`] if `systemctl start` exits non-zero.
#[cfg(feature = "client")]
pub fn start(service: &str) -> Result<()> {
    let runner = toride_runner::DuctRunner;
    let spec = CommandSpec::new("systemctl").arg("start").arg(service);
    let output = runner.run(&spec)?;
    if output.success {
        Ok(())
    } else {
        let stderr = output.stderr.trim();
        let detail = if stderr.is_empty() {
            format!("systemctl start {service} failed")
        } else {
            format!("systemctl start {service} failed: {stderr}")
        };
        Err(Error::CommandFailed(detail))
    }
}

/// Stop the given service unit.
///
/// # Errors
///
/// Returns [`Error::CommandFailed`] if `systemctl stop` exits non-zero.
#[cfg(feature = "client")]
pub fn stop(service: &str) -> Result<()> {
    let runner = toride_runner::DuctRunner;
    let spec = CommandSpec::new("systemctl").arg("stop").arg(service);
    let output = runner.run(&spec)?;
    if output.success {
        Ok(())
    } else {
        let stderr = output.stderr.trim();
        let detail = if stderr.is_empty() {
            format!("systemctl stop {service} failed")
        } else {
            format!("systemctl stop {service} failed: {stderr}")
        };
        Err(Error::CommandFailed(detail))
    }
}

/// Restart the given service unit.
///
/// # Errors
///
/// Returns [`Error::CommandFailed`] if `systemctl restart` exits non-zero.
#[cfg(feature = "client")]
pub fn restart(service: &str) -> Result<()> {
    let runner = toride_runner::DuctRunner;
    let spec = CommandSpec::new("systemctl").arg("restart").arg(service);
    let output = runner.run(&spec)?;
    if output.success {
        Ok(())
    } else {
        let stderr = output.stderr.trim();
        let detail = if stderr.is_empty() {
            format!("systemctl restart {service} failed")
        } else {
            format!("systemctl restart {service} failed: {stderr}")
        };
        Err(Error::CommandFailed(detail))
    }
}
