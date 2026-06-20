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
    /// For the systemd backend this performs a **real** probe: it first checks
    /// that systemd is the running init system on this host (via
    /// [`crate::systemd::detect`]); if systemd is absent it honestly reports
    /// `Ok(false)` and records an informational note (see
    /// [`schedule_note`](Self::schedule_note)). When systemd is present it
    /// looks for the job's timer unit (and, more broadly, any backup-related
    /// timer) via `systemctl cat` / `systemctl list-timers`.
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

    /// Return an informational note explaining the most recent schedule probe.
    ///
    /// Performs a fresh systemd detection probe and returns `"systemd not
    /// detected"` when systemd is absent on this host, or an empty string when
    /// systemd is present (in which case `is_installed` reflects the real
    /// unit-file state). This lets the UI surface *why* a schedule read as
    /// `false` without changing the `is_installed` return type.
    pub fn schedule_note(&self) -> String {
        let detected = crate::systemd::detect();
        if !detected.available {
            detected.note
        } else {
            String::new()
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
        // Real probe: if systemd isn't the running init system on this host,
        // honestly report "no schedule installed". The accompanying note is
        // available via [`Self::schedule_note`].
        let detected = crate::systemd::detect();
        if !detected.available {
            tracing::debug!(note = %detected.note, "systemd absent; reporting schedule_installed=false");
            return Ok(false);
        }
        // systemd is present: look for the job-specific timer unit, and fall
        // back to "any backup timer installed" so a host using restic/borg
        // timers still reads as scheduled.
        let job_unit = format!("{}{}.timer", "toride-backup-", name);
        let probe = crate::systemd::probe_timer(&job_unit);
        if probe.installed {
            return Ok(true);
        }
        // Broader discovery: any backup-related timer unit on the host counts.
        Ok(crate::systemd::any_backup_timer_installed())
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
