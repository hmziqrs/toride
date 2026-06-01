//! Config file read/write with atomic writes.
//!
//! Reads and writes update configuration files using [`toride_fs::atomic_write`]
//! to ensure no partial writes are left on disk if the process crashes.

use std::path::Path;

use tracing::info;

use crate::error::{Error, Result};
use crate::paths::UpdatePaths;
use crate::spec::UpdateSpec;

// ---------------------------------------------------------------------------
// ConfigManager
// ---------------------------------------------------------------------------

/// Read and write automatic update configuration files.
///
/// Uses [`toride_fs::atomic_write`] to ensure configuration changes are
//! atomic. Creates backups of existing files before overwriting.
pub struct ConfigManager {
    paths: UpdatePaths,
}

impl ConfigManager {
    /// Create a new config manager with auto-detected paths.
    #[must_use]
    pub fn new() -> Self {
        Self {
            paths: UpdatePaths::detect(),
        }
    }

    /// Create a config manager with explicit paths.
    #[must_use]
    pub fn with_paths(paths: UpdatePaths) -> Self {
        Self { paths }
    }

    /// Read the current update configuration and return an [`UpdateSpec`].
    ///
    /// Parses the existing config files on disk and constructs a spec
    /// reflecting the current state.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ConfigParse`] if the config files cannot be parsed,
    /// or [`Error::Io`] if they cannot be read.
    pub fn read_current(&self) -> Result<UpdateSpec> {
        info!("Reading current update configuration");
        // TODO: Parse actual config files from disk.
        Ok(UpdateSpec::default())
    }

    /// Write an [`UpdateSpec`] to disk as configuration files.
    ///
    /// Backs up existing files, renders the spec into backend-specific config
    /// format, and atomically writes the results.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ConfigWrite`] if the write fails, or [`Error::Io`]
    /// if the backup fails.
    pub fn write_spec(&self, spec: &UpdateSpec) -> Result<()> {
        info!("Writing update configuration to disk");

        // Detect which backend to use.
        let pkg_mgr = crate::detect::detect_package_manager();

        match pkg_mgr {
            crate::detect::PackageManager::Apt => self.write_apt_config(spec)?,
            crate::detect::PackageManager::Dnf => self.write_dnf_config(spec)?,
            crate::detect::PackageManager::Unknown => {
                return Err(Error::PackageDetection(
                    "no supported package manager detected".into(),
                ));
            }
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Backend-specific writers
    // -----------------------------------------------------------------------

    fn write_apt_config(&self, spec: &UpdateSpec) -> Result<()> {
        // Backup existing configs.
        crate::backup::backup_config(&self.paths.auto_upgrades_conf)?;
        crate::backup::backup_config(&self.paths.auto_upgrades_enabled)?;

        // Render configs.
        let auto_upgrades = crate::render::render_auto_upgrades_conf(spec);
        let apt_conf = crate::render::render_apt_conf(spec);

        // Atomic write.
        toride_fs::atomic_write(&self.paths.auto_upgrades_conf, &auto_upgrades)
            .map_err(|e| Error::ConfigWrite(format!("failed to write 50unattended-upgrades: {e}")))?;

        toride_fs::atomic_write(&self.paths.auto_upgrades_enabled, &apt_conf)
            .map_err(|e| Error::ConfigWrite(format!("failed to write 20auto-upgrades: {e}")))?;

        Ok(())
    }

    fn write_dnf_config(&self, spec: &UpdateSpec) -> Result<()> {
        // Backup existing config.
        crate::backup::backup_config(&self.paths.dnf_automatic_conf)?;

        // Render config.
        let dnf_conf = crate::render::render_dnf_automatic_conf(spec);

        // Atomic write.
        toride_fs::atomic_write(&self.paths.dnf_automatic_conf, &dnf_conf)
            .map_err(|e| Error::ConfigWrite(format!("failed to write automatic.conf: {e}")))?;

        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
