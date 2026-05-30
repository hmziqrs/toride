//! Collection presets for different use cases.
//!
//! Presets control which metrics are collected, allowing callers to
//! optimize for their specific needs (minimal overhead vs full diagnostics).
//!
//! # Preset comparison
//!
//! | Feature            | Minimal | `TaskManager` | Diagnostics | `ServerMonitoring` | `PrivacySafe` |
//! |--------------------|:-------:|:-------------:|:-----------:|:------------------:|:-------------:|
//! | CPU usage          | Yes     | Yes         | Yes         | Yes              | Yes         |
//! | Per-core CPU       | No      | Yes         | Yes         | No               | No          |
//! | Memory             | Yes     | Yes         | Yes         | Yes              | Yes         |
//! | Swap               | No      | No          | Yes         | Yes              | No          |
//! | Disk (root)        | Yes     | Yes         | Yes         | Yes              | No          |
//! | All disks          | No      | Yes         | Yes         | No               | No          |
//! | Network            | Yes     | Yes         | Yes         | Yes              | No          |
//! | Network interfaces | No      | No          | Yes         | Yes              | No          |
//! | Load average       | Yes     | Yes         | Yes         | Yes              | No          |
//! | Uptime             | Yes     | Yes         | Yes         | Yes              | No          |
//! | Hostname           | Yes     | Yes         | Yes         | Yes              | No          |
//! | OS info            | Yes     | Yes         | Yes         | Yes              | Yes         |
//! | Sensors            | No      | Yes         | Yes         | No               | No          |
//! | Processes          | No      | Yes         | Yes         | No               | No          |
//! | GPU                | No      | No          | Yes         | No               | Yes         |
//! | Battery            | No      | No          | Yes         | No               | No          |
//!
//! # Choosing a preset
//!
//! - **Minimal**: Lightweight monitoring with basic resource usage. Best for
//!   dashboards that only need CPU, memory, disk, and network totals.
//! - **Task Manager**: Interactive process monitoring. Includes per-core CPU,
//!   all disks, sensors, and process lists.
//! - **Diagnostics**: Full system report. Includes everything except secrets.
//!   This is the default preset.
//! - **`ServerMonitoring`**: Headless server monitoring. Includes network
//!   interfaces and swap, but omits sensors and processes for lower overhead.
//! - **`PrivacySafeBugReport`**: Redacted data safe for sharing in bug reports.
//!   Only includes OS info, CPU family, memory total, and GPU model.
//!
//! # Examples
//!
//! ```no_run
//! use toride::status::presets::Preset;
//!
//! let preset = Preset::TaskManager;
//! println!("Preset: {preset}");
//!
//! if preset.includes_processes() {
//!     println!("Process monitoring enabled");
//! }
//! ```

use std::fmt;

use serde::Serialize;

/// Collection preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub enum Preset {
    /// Minimal: CPU, memory, disk, network, uptime.
    Minimal,
    /// Task manager: CPU, memory, disks, network, processes, sensors.
    TaskManager,
    /// Full diagnostics: everything except secrets.
    #[default]
    Diagnostics,
    /// Server monitoring: CPU, memory, disk I/O, network I/O, uptime.
    ServerMonitoring,
    /// Privacy-safe bug report: OS, CPU family, memory total, GPU model.
    PrivacySafeBugReport,
}

impl Preset {
    /// Whether this preset includes per-core CPU data.
    ///
    /// Returns `true` for `TaskManager` and `Diagnostics` presets.
    ///
    /// # Examples
    ///
    /// ```
    /// use toride::status::presets::Preset;
    ///
    /// assert!(Preset::TaskManager.includes_per_core_cpu());
    /// assert!(!Preset::Minimal.includes_per_core_cpu());
    /// ```
    #[must_use]
    pub const fn includes_per_core_cpu(self) -> bool {
        matches!(self, Self::TaskManager | Self::Diagnostics)
    }

    /// Whether this preset includes swap data.
    ///
    /// Returns `true` for `Diagnostics` and `ServerMonitoring` presets.
    ///
    /// # Examples
    ///
    /// ```
    /// use toride::status::presets::Preset;
    ///
    /// assert!(Preset::Diagnostics.includes_swap());
    /// assert!(!Preset::Minimal.includes_swap());
    /// ```
    #[must_use]
    pub const fn includes_swap(self) -> bool {
        matches!(self, Self::Diagnostics | Self::ServerMonitoring)
    }

    /// Whether this preset includes sensor data.
    ///
    /// Returns `true` for `TaskManager` and `Diagnostics` presets.
    ///
    /// # Examples
    ///
    /// ```
    /// use toride::status::presets::Preset;
    ///
    /// assert!(Preset::Diagnostics.includes_sensors());
    /// assert!(!Preset::ServerMonitoring.includes_sensors());
    /// ```
    #[must_use]
    pub const fn includes_sensors(self) -> bool {
        matches!(self, Self::TaskManager | Self::Diagnostics)
    }

    /// Whether this preset includes process data.
    ///
    /// Returns `true` for `TaskManager` and `Diagnostics` presets.
    ///
    /// # Examples
    ///
    /// ```
    /// use toride::status::presets::Preset;
    ///
    /// assert!(Preset::TaskManager.includes_processes());
    /// assert!(!Preset::Minimal.includes_processes());
    /// ```
    #[must_use]
    pub const fn includes_processes(self) -> bool {
        matches!(self, Self::TaskManager | Self::Diagnostics)
    }

    /// Whether this preset includes network interface details.
    ///
    /// Returns `true` for `Diagnostics` and `ServerMonitoring` presets.
    ///
    /// # Examples
    ///
    /// ```
    /// use toride::status::presets::Preset;
    ///
    /// assert!(Preset::ServerMonitoring.includes_network_interfaces());
    /// assert!(!Preset::Minimal.includes_network_interfaces());
    /// ```
    #[must_use]
    pub const fn includes_network_interfaces(self) -> bool {
        matches!(self, Self::Diagnostics | Self::ServerMonitoring)
    }

    /// Whether this preset includes all disk partitions.
    ///
    /// Returns `true` for `Diagnostics` and `TaskManager` presets.
    /// When `false`, only the root disk is included.
    ///
    /// # Examples
    ///
    /// ```
    /// use toride::status::presets::Preset;
    ///
    /// assert!(Preset::Diagnostics.includes_all_disks());
    /// assert!(!Preset::ServerMonitoring.includes_all_disks());
    /// ```
    #[must_use]
    pub const fn includes_all_disks(self) -> bool {
        matches!(self, Self::Diagnostics | Self::TaskManager)
    }

    /// Whether this preset includes OS info.
    ///
    /// Always returns `true` — OS information is useful in every context.
    ///
    /// # Examples
    ///
    /// ```
    /// use toride::status::presets::Preset;
    ///
    /// assert!(Preset::Minimal.includes_os_info());
    /// assert!(Preset::Diagnostics.includes_os_info());
    /// ```
    #[must_use]
    pub const fn includes_os_info(self) -> bool {
        true // always useful
    }

    /// The human-readable name of this preset.
    ///
    /// # Examples
    ///
    /// ```
    /// use toride::status::presets::Preset;
    ///
    /// assert_eq!(Preset::Minimal.name(), "Minimal");
    /// assert_eq!(Preset::Diagnostics.name(), "Diagnostics");
    /// assert_eq!(Preset::ServerMonitoring.name(), "Server Monitoring");
    /// ```
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Minimal => "Minimal",
            Self::TaskManager => "Task Manager",
            Self::Diagnostics => "Diagnostics",
            Self::ServerMonitoring => "Server Monitoring",
            Self::PrivacySafeBugReport => "Privacy-Safe Bug Report",
        }
    }
}

impl fmt::Display for Preset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_diagnostics() {
        assert_eq!(Preset::default(), Preset::Diagnostics);
    }

    #[test]
    fn minimal_excludes_per_core_cpu() {
        assert!(!Preset::Minimal.includes_per_core_cpu());
    }

    #[test]
    fn task_manager_includes_processes() {
        assert!(Preset::TaskManager.includes_processes());
    }

    #[test]
    fn diagnostics_includes_everything() {
        let p = Preset::Diagnostics;
        assert!(p.includes_per_core_cpu());
        assert!(p.includes_swap());
        assert!(p.includes_sensors());
        assert!(p.includes_processes());
        assert!(p.includes_network_interfaces());
        assert!(p.includes_all_disks());
        assert!(p.includes_os_info());
    }

    #[test]
    fn preset_names() {
        assert_eq!(Preset::Minimal.name(), "Minimal");
        assert_eq!(Preset::TaskManager.name(), "Task Manager");
    }

    #[test]
    fn preset_display() {
        assert_eq!(format!("{}", Preset::Minimal), "Minimal");
    }

    #[test]
    fn serialize_to_json() {
        assert!(serde_json::to_string(&Preset::Diagnostics).is_ok());
    }

    // --- Comprehensive preset coverage ---

    #[test]
    fn minimal_includes_per_core_cpu() {
        assert!(!Preset::Minimal.includes_per_core_cpu());
    }

    #[test]
    fn minimal_includes_swap() {
        assert!(!Preset::Minimal.includes_swap());
    }

    #[test]
    fn minimal_includes_sensors() {
        assert!(!Preset::Minimal.includes_sensors());
    }

    #[test]
    fn minimal_includes_processes() {
        assert!(!Preset::Minimal.includes_processes());
    }

    #[test]
    fn minimal_includes_network_interfaces() {
        assert!(!Preset::Minimal.includes_network_interfaces());
    }

    #[test]
    fn minimal_includes_all_disks() {
        assert!(!Preset::Minimal.includes_all_disks());
    }

    #[test]
    fn minimal_includes_os_info() {
        assert!(Preset::Minimal.includes_os_info());
    }

    #[test]
    fn task_manager_includes_per_core_cpu() {
        assert!(Preset::TaskManager.includes_per_core_cpu());
    }

    #[test]
    fn task_manager_includes_swap() {
        assert!(!Preset::TaskManager.includes_swap());
    }

    #[test]
    fn task_manager_includes_sensors() {
        assert!(Preset::TaskManager.includes_sensors());
    }

    #[test]
    fn task_manager_includes_network_interfaces() {
        assert!(!Preset::TaskManager.includes_network_interfaces());
    }

    #[test]
    fn task_manager_includes_all_disks() {
        assert!(Preset::TaskManager.includes_all_disks());
    }

    #[test]
    fn task_manager_includes_os_info() {
        assert!(Preset::TaskManager.includes_os_info());
    }

    #[test]
    fn diagnostics_includes_per_core_cpu() {
        assert!(Preset::Diagnostics.includes_per_core_cpu());
    }

    #[test]
    fn diagnostics_includes_swap() {
        assert!(Preset::Diagnostics.includes_swap());
    }

    #[test]
    fn diagnostics_includes_sensors() {
        assert!(Preset::Diagnostics.includes_sensors());
    }

    #[test]
    fn diagnostics_includes_network_interfaces() {
        assert!(Preset::Diagnostics.includes_network_interfaces());
    }

    #[test]
    fn diagnostics_includes_all_disks() {
        assert!(Preset::Diagnostics.includes_all_disks());
    }

    #[test]
    fn diagnostics_includes_os_info() {
        assert!(Preset::Diagnostics.includes_os_info());
    }

    #[test]
    fn server_monitoring_includes_per_core_cpu() {
        assert!(!Preset::ServerMonitoring.includes_per_core_cpu());
    }

    #[test]
    fn server_monitoring_includes_swap() {
        assert!(Preset::ServerMonitoring.includes_swap());
    }

    #[test]
    fn server_monitoring_includes_sensors() {
        assert!(!Preset::ServerMonitoring.includes_sensors());
    }

    #[test]
    fn server_monitoring_includes_processes() {
        assert!(!Preset::ServerMonitoring.includes_processes());
    }

    #[test]
    fn server_monitoring_includes_network_interfaces() {
        assert!(Preset::ServerMonitoring.includes_network_interfaces());
    }

    #[test]
    fn server_monitoring_includes_all_disks() {
        assert!(!Preset::ServerMonitoring.includes_all_disks());
    }

    #[test]
    fn server_monitoring_includes_os_info() {
        assert!(Preset::ServerMonitoring.includes_os_info());
    }

    #[test]
    fn privacy_safe_bug_report_includes_per_core_cpu() {
        assert!(!Preset::PrivacySafeBugReport.includes_per_core_cpu());
    }

    #[test]
    fn privacy_safe_bug_report_includes_swap() {
        assert!(!Preset::PrivacySafeBugReport.includes_swap());
    }

    #[test]
    fn privacy_safe_bug_report_includes_sensors() {
        assert!(!Preset::PrivacySafeBugReport.includes_sensors());
    }

    #[test]
    fn privacy_safe_bug_report_includes_processes() {
        assert!(!Preset::PrivacySafeBugReport.includes_processes());
    }

    #[test]
    fn privacy_safe_bug_report_includes_network_interfaces() {
        assert!(!Preset::PrivacySafeBugReport.includes_network_interfaces());
    }

    #[test]
    fn privacy_safe_bug_report_includes_all_disks() {
        assert!(!Preset::PrivacySafeBugReport.includes_all_disks());
    }

    #[test]
    fn privacy_safe_bug_report_includes_os_info() {
        assert!(Preset::PrivacySafeBugReport.includes_os_info());
    }

    // --- Preset name edge cases ---

    #[test]
    fn all_presets_have_unique_names() {
        let presets = [
            Preset::Minimal,
            Preset::TaskManager,
            Preset::Diagnostics,
            Preset::ServerMonitoring,
            Preset::PrivacySafeBugReport,
        ];
        let names: Vec<&str> = presets.iter().map(|p| p.name()).collect();
        let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(names.len(), unique.len(), "preset names must be unique");
    }

    #[test]
    fn all_presets_have_non_empty_names() {
        let presets = [
            Preset::Minimal,
            Preset::TaskManager,
            Preset::Diagnostics,
            Preset::ServerMonitoring,
            Preset::PrivacySafeBugReport,
        ];
        for preset in presets {
            assert!(
                !preset.name().is_empty(),
                "{preset:?} has an empty name"
            );
        }
    }

    // --- Display edge cases ---

    #[test]
    fn all_presets_display_correctly() {
        let cases = [
            (Preset::Minimal, "Minimal"),
            (Preset::TaskManager, "Task Manager"),
            (Preset::Diagnostics, "Diagnostics"),
            (Preset::ServerMonitoring, "Server Monitoring"),
            (Preset::PrivacySafeBugReport, "Privacy-Safe Bug Report"),
        ];
        for (preset, expected) in cases {
            assert_eq!(format!("{preset}"), expected);
        }
    }
}
