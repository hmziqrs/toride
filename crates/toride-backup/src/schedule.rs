//! Backup scheduling via systemd timers or cron.
//!
//! Manages the creation, validation, and lifecycle of backup schedules.
//! Supports systemd timer units (preferred on modern Linux) and cron
//! fallback for other systems.

use crate::spec::Schedule;
use crate::{Error, Result};

// ---------------------------------------------------------------------------
// ScheduleBackend
// ---------------------------------------------------------------------------

/// Backend used for scheduling backups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScheduleBackend {
    /// Use systemd timer units (preferred on modern Linux).
    #[default]
    SystemdTimer,
    /// Use cron (crontab entries).
    Cron,
}

// ---------------------------------------------------------------------------
// ScheduleManager
// ---------------------------------------------------------------------------

/// Manages backup schedule installation and removal.
///
/// Creates and manages systemd timer units or cron entries for backup jobs.
/// Each backup spec maps to one schedule entry.
pub struct ScheduleManager {
    /// Which scheduling backend to use.
    backend: ScheduleBackend,
}

impl ScheduleManager {
    /// Create a schedule manager targeting the default backend (systemd).
    pub fn new() -> Self {
        Self {
            backend: ScheduleBackend::default(),
        }
    }

    /// Create a schedule manager targeting a specific backend.
    pub fn with_backend(backend: ScheduleBackend) -> Self {
        Self { backend }
    }

    /// Install a schedule for the given backup job name.
    ///
    /// For systemd timers, this creates a `.service` and `.timer` unit pair.
    /// For cron, this appends an entry to the crontab.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ScheduleError`] if the schedule cannot be installed.
    pub fn install(&self, name: &str, schedule: &Schedule) -> Result<()> {
        schedule.validate()?;

        match self.backend {
            ScheduleBackend::SystemdTimer => {
                self.install_systemd_timer(name, schedule)
            }
            ScheduleBackend::Cron => {
                self.install_cron(name, schedule)
            }
        }
    }

    /// Remove a schedule for the given backup job name.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ScheduleError`] if the schedule cannot be removed.
    pub fn remove(&self, name: &str) -> Result<()> {
        match self.backend {
            ScheduleBackend::SystemdTimer => {
                self.remove_systemd_timer(name)
            }
            ScheduleBackend::Cron => {
                self.remove_cron(name)
            }
        }
    }

    /// Check whether a schedule is installed for the given backup job.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ScheduleError`] if the check fails.
    pub fn is_installed(&self, name: &str) -> Result<bool> {
        match self.backend {
            ScheduleBackend::SystemdTimer => {
                self.is_systemd_timer_installed(name)
            }
            ScheduleBackend::Cron => {
                self.is_cron_installed(name)
            }
        }
    }

    // -----------------------------------------------------------------------
    // Systemd timer implementation
    // -----------------------------------------------------------------------

    fn install_systemd_timer(&self, name: &str, schedule: &Schedule) -> Result<()> {
        // TODO: generate and write systemd .service and .timer unit files.
        tracing::info!(
            name = %name,
            cron = %schedule.cron,
            "installing systemd timer (not yet implemented)"
        );
        Err(Error::ScheduleError(
            "systemd timer installation not yet implemented".into(),
        ))
    }

    fn remove_systemd_timer(&self, name: &str) -> Result<()> {
        // TODO: stop, disable, and remove systemd unit files.
        tracing::info!(
            name = %name,
            "removing systemd timer (not yet implemented)"
        );
        Err(Error::ScheduleError(
            "systemd timer removal not yet implemented".into(),
        ))
    }

    fn is_systemd_timer_installed(&self, name: &str) -> Result<bool> {
        // TODO: check if systemd timer unit file exists.
        let _ = name;
        Ok(false)
    }

    // -----------------------------------------------------------------------
    // Cron implementation
    // -----------------------------------------------------------------------

    fn install_cron(&self, name: &str, schedule: &Schedule) -> Result<()> {
        // TODO: append cron entry.
        tracing::info!(
            name = %name,
            cron = %schedule.cron,
            "installing cron entry (not yet implemented)"
        );
        Err(Error::ScheduleError(
            "cron installation not yet implemented".into(),
        ))
    }

    fn remove_cron(&self, name: &str) -> Result<()> {
        // TODO: remove cron entry.
        tracing::info!(
            name = %name,
            "removing cron entry (not yet implemented)"
        );
        Err(Error::ScheduleError(
            "cron removal not yet implemented".into(),
        ))
    }

    fn is_cron_installed(&self, name: &str) -> Result<bool> {
        // TODO: check if cron entry exists.
        let _ = name;
        Ok(false)
    }
}
