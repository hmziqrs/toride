//! Service management for user-related system daemons.
//!
//! Provides functions to check and manage the status of services that are
//! related to user authentication and access control (e.g. `sshd`).

use crate::{Error, Result};

/// Check if a systemd service is active.
///
/// Executes `systemctl is-active <service>`.
///
/// # Errors
///
/// - [`Error::BinaryNotFound`] if `systemctl` is not on `$PATH`.
/// - [`Error::CommandFailed`] if `systemctl` returns an unexpected error.
pub fn is_service_active(service: &str) -> Result<bool> {
    let systemctl =
        which::which("systemctl").map_err(|_| Error::BinaryNotFound("systemctl".into()))?;

    let result = duct::cmd(&systemctl, ["is-active", service])
        .stderr_capture()
        .read();

    match result {
        Ok(output) => Ok(output.trim() == "active"),
        Err(_) => Ok(false),
    }
}

/// Restart a systemd service.
///
/// Executes `systemctl restart <service>`.
///
/// # Errors
///
/// - [`Error::BinaryNotFound`] if `systemctl` is not on `$PATH`.
/// - [`Error::CommandFailed`] if `systemctl` returns a non-zero exit code.
pub fn restart_service(service: &str) -> Result<()> {
    let systemctl =
        which::which("systemctl").map_err(|_| Error::BinaryNotFound("systemctl".into()))?;

    duct::cmd(&systemctl, ["restart", service])
        .stderr_to_stdout()
        .read()
        .map_err(|e| Error::CommandFailed {
            program: "systemctl".to_owned(),
            code: None,
            stderr: e.to_string(),
        })?;

    tracing::info!("restarted service {service}");
    Ok(())
}

/// Reload a systemd service (send SIGHUP).
///
/// Executes `systemctl reload <service>`.
///
/// # Errors
///
/// - [`Error::BinaryNotFound`] if `systemctl` is not on `$PATH`.
/// - [`Error::CommandFailed`] if `systemctl` returns a non-zero exit code.
pub fn reload_service(service: &str) -> Result<()> {
    let systemctl =
        which::which("systemctl").map_err(|_| Error::BinaryNotFound("systemctl".into()))?;

    duct::cmd(&systemctl, ["reload", service])
        .stderr_to_stdout()
        .read()
        .map_err(|e| Error::CommandFailed {
            program: "systemctl".to_owned(),
            code: None,
            stderr: e.to_string(),
        })?;

    tracing::info!("reloaded service {service}");
    Ok(())
}

/// Ensure `sshd` is running after PAM or authentication changes.
///
/// Checks if `sshd` is active, and if so, reloads it to pick up
/// configuration changes.
///
/// # Errors
///
/// Returns any error from service status checks or reload operations.
pub fn reload_sshd_if_active() -> Result<()> {
    if is_service_active("sshd")? {
        reload_service("sshd")?;
    }
    Ok(())
}

/// Service manager handle for user-related services.
pub struct ServiceManager;

impl ServiceManager {
    /// Create a new service manager.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Check if a service is active.
    pub fn is_active(&self, service: &str) -> Result<bool> {
        is_service_active(service)
    }

    /// Restart a service.
    pub fn restart(&self, service: &str) -> Result<()> {
        restart_service(service)
    }

    /// Reload a service.
    pub fn reload(&self, service: &str) -> Result<()> {
        reload_service(service)
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}
