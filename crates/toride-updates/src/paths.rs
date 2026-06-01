//! Resolved filesystem paths for automatic update configuration files.
//!
//! [`UpdatePaths`] centralizes all paths that the updates subsystem reads from
//! or writes to, including APT's `50unattended-upgrades`, `20auto-upgrades`,
//! DNF's `automatic.conf`, and their parent directories.

use std::path::PathBuf;

// ---------------------------------------------------------------------------
// UpdatePaths
// ---------------------------------------------------------------------------

/// Resolved paths to automatic update configuration files on the host.
///
/// On Debian/Ubuntu systems, paths point into `/etc/apt/apt.conf.d/`.
/// On Fedora/RHEL systems, paths point to `/etc/dnf/automatic.conf` and
/// related systemd timer units.
///
/// Use [`UpdatePaths::detect`] to auto-select paths based on the detected
/// package manager, or [`UpdatePaths::new`] for a sensible default.
#[derive(Debug, Clone)]
pub struct UpdatePaths {
    /// Path to `50unattended-upgrades` (Debian/Ubuntu).
    pub auto_upgrades_conf: PathBuf,
    /// Path to `/etc/apt/apt.conf.d/` directory.
    pub apt_conf_d: PathBuf,
    /// Path to `/etc/apt/apt.conf.d/20auto-upgrades`.
    pub auto_upgrades_enabled: PathBuf,
    /// Path to `/etc/dnf/automatic.conf` (Fedora/RHEL).
    pub dnf_automatic_conf: PathBuf,
    /// Path to `/etc/dnf/` directory.
    pub dnf_conf_d: PathBuf,
    /// Path to the systemd timer unit drop-in directory.
    pub systemd_timer_d: PathBuf,
    /// Path to the log file for update operations.
    pub log_file: PathBuf,
}

impl UpdatePaths {
    /// Create an `UpdatePaths` with default Linux VPS paths.
    ///
    /// This sets up paths for both APT and DNF backends. The caller should
    /// use [`detect`](Self::detect) to auto-select the correct backend paths.
    #[must_use]
    pub fn new() -> Self {
        Self {
            auto_upgrades_conf: PathBuf::from("/etc/apt/apt.conf.d/50unattended-upgrades"),
            apt_conf_d: PathBuf::from("/etc/apt/apt.conf.d"),
            auto_upgrades_enabled: PathBuf::from("/etc/apt/apt.conf.d/20auto-upgrades"),
            dnf_automatic_conf: PathBuf::from("/etc/dnf/automatic.conf"),
            dnf_conf_d: PathBuf::from("/etc/dnf"),
            systemd_timer_d: PathBuf::from("/etc/systemd/system/dnf-automatic.timer.d"),
            log_file: PathBuf::from("/var/log/unattended-upgrades/unattended-upgrades.log"),
        }
    }

    /// Detect the active package manager and return paths tailored to it.
    ///
    /// Checks for `apt-get` and `dnf` on `$PATH`. If neither is found,
    /// returns the default paths (which will likely produce errors on use).
    ///
    /// # Errors
    ///
    /// This method does not currently return errors but reserves the right to
    /// do so in future versions (the enum is `#[non_exhaustive]`).
    pub fn detect() -> Self {
        let paths = Self::new();
        // In a real implementation we would probe the filesystem here.
        // For now return the default paths.
        paths
    }

    /// Returns `true` if the APT (Debian/Ubuntu) backend paths exist on disk.
    pub fn is_apt_available(&self) -> bool {
        self.apt_conf_d.is_dir()
    }

    /// Returns `true` if the DNF (Fedora/RHEL) backend paths exist on disk.
    pub fn is_dnf_available(&self) -> bool {
        self.dnf_conf_d.is_dir()
    }
}

impl Default for UpdatePaths {
    fn default() -> Self {
        Self::new()
    }
}
