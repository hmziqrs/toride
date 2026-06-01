//! Systemd timer / cron schedule management for automatic updates.
//!
//! Manages the schedule that triggers automatic update checks and applies.
//! On systems with systemd, uses timer units. Falls back to cron otherwise.

use tracing::info;

use crate::detect::PackageManager;
use crate::error::{Error, Result};
use crate::paths::UpdatePaths;
use crate::spec::Schedule;

// ---------------------------------------------------------------------------
// ScheduleManager
// ---------------------------------------------------------------------------

/// Manages the automatic update schedule.
///
/// On APT systems, configures `APT::Periodic` settings and optionally a
/// systemd timer. On DNF systems, manages the `dnf-automatic.timer` unit.
pub struct ScheduleManager<'a> {
    runner: &'a dyn toride_runner::Runner,
    paths: UpdatePaths,
    pkg_mgr: PackageManager,
}

impl<'a> ScheduleManager<'a> {
    /// Create a new schedule manager with the given runner.
    pub fn new(runner: &'a dyn toride_runner::Runner) -> Self {
        Self {
            runner,
            paths: UpdatePaths::detect(),
            pkg_mgr: crate::detect::detect_package_manager(),
        }
    }

    /// Set the update schedule.
    ///
    /// Configures the appropriate timer or cron job for the given schedule.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ScheduleConfig`] if the schedule cannot be configured.
    pub fn set_schedule(&self, schedule: &Schedule) -> Result<()> {
        info!("Setting update schedule: {schedule}");

        match self.pkg_mgr {
            PackageManager::Apt => self.set_apt_schedule(schedule),
            PackageManager::Dnf => self.set_dnf_schedule(schedule),
            PackageManager::Unknown => Err(Error::ScheduleConfig(
                "no supported package manager for schedule setup".into(),
            )),
        }
    }

    /// Get the currently configured schedule, if any.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ScheduleConfig`] if the schedule cannot be determined.
    pub fn get_schedule(&self) -> Result<Option<Schedule>> {
        info!("Querying current update schedule");

        match self.pkg_mgr {
            PackageManager::Apt => self.get_apt_schedule(),
            PackageManager::Dnf => self.get_dnf_schedule(),
            PackageManager::Unknown => Ok(None),
        }
    }

    /// Remove the configured schedule.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ScheduleConfig`] if the schedule cannot be removed.
    pub fn remove_schedule(&self) -> Result<()> {
        info!("Removing update schedule");
        // TODO: Disable and remove the systemd timer or cron job.
        Ok(())
    }

    // -----------------------------------------------------------------------
    // APT schedule helpers
    // -----------------------------------------------------------------------

    fn set_apt_schedule(&self, schedule: &Schedule) -> Result<()> {
        let interval = match schedule {
            Schedule::Daily => "1",
            Schedule::Weekly => "7",
            Schedule::Monthly => "30",
            Schedule::Custom(expr) => {
                // For custom schedules, we use a systemd timer instead of
                // APT::Periodic.
                self.create_systemd_timer(expr)?;
                return Ok(());
            }
        };

        // Write APT::Periodic::Update-Package-Lists interval.
        let apt_conf = format!(
            "APT::Periodic::Update-Package-Lists \"{interval}\";\n\
             APT::Periodic::Unattended-Upgrade \"1\";\n"
        );

        toride_fs::atomic_write(&self.paths.auto_upgrades_enabled, &apt_conf)
            .map_err(|e| Error::ScheduleConfig(format!("failed to write APT schedule: {e}")))?;

        Ok(())
    }

    fn get_apt_schedule(&self) -> Result<Option<Schedule>> {
        if !self.paths.auto_upgrades_enabled.exists() {
            return Ok(None);
        }
        // TODO: Parse the APT::Periodic interval from the config file.
        Ok(Some(Schedule::Daily))
    }

    // -----------------------------------------------------------------------
    // DNF schedule helpers
    // -----------------------------------------------------------------------

    fn set_dnf_schedule(&self, schedule: &Schedule) -> Result<()> {
        let calendar = schedule_to_calendar(schedule);
        self.create_systemd_timer(&calendar)
    }

    fn get_dnf_schedule(&self) -> Result<Option<Schedule>> {
        // TODO: Parse the systemd timer OnCalendar directive.
        let _ = &self.paths;
        Ok(None)
    }

    // -----------------------------------------------------------------------
    // Systemd timer
    // -----------------------------------------------------------------------

    fn create_systemd_timer(&self, calendar_expr: &str) -> Result<()> {
        info!("Creating systemd timer with OnCalendar={calendar_expr}");

        let timer_unit = format!(
            "[Unit]\n\
             Description=Automatic security updates (toride-managed)\n\n\
             [Timer]\n\
             OnCalendar={calendar_expr}\n\
             Persistent=true\n\
             RandomizedDelaySec=300\n\n\
             [Install]\n\
             WantedBy=timers.target\n"
        );

        // Determine timer name based on package manager.
        let timer_name = match self.pkg_mgr {
            PackageManager::Dnf => "dnf-automatic.timer",
            _ => "toride-updates.timer",
        };

        let timer_path = self.paths.systemd_timer_d.join(format!("{timer_name}.conf"));

        crate::backup::backup_config(&timer_path)?;

        toride_fs::atomic_write(&timer_path, &timer_unit)
            .map_err(|e| Error::ScheduleConfig(format!("failed to write timer unit: {e}")))?;

        // Reload systemd.
        self.runner
            .run_stderr_ok(&["systemctl", "daemon-reload"])
            .map_err(|e| Error::CommandFailed(format!("systemctl daemon-reload failed: {e}")))?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a [`Schedule`] to a systemd calendar expression.
fn schedule_to_calendar(schedule: &Schedule) -> String {
    match schedule {
        Schedule::Daily => "daily".to_owned(),
        Schedule::Weekly => "weekly".to_owned(),
        Schedule::Monthly => "monthly".to_owned(),
        Schedule::Custom(expr) => expr.clone(),
    }
}
