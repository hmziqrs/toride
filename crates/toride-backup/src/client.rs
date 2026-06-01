//! High-level backup client facade.
//!
//! [`BackupClient`] is the main entry point for the `client` feature. It
//! composes a command runner, system paths, and delegates to sub-modules
//! for backup operations, restore workflows, scheduling, and doctor
//! diagnostics.
//!
//! # Example
//!
//! ```ignore
//! use toride_backup::BackupClient;
//!
//! let client = BackupClient::system()?;
//! let report = client.backup(&spec)?;
//! ```

use std::path::PathBuf;

use crate::backup;
use crate::paths::BackupPaths;
use crate::report::{BackupReport, PruneReport};
use crate::restore::{RestoreManager, RestoreOptions};
use crate::schedule::ScheduleManager;
use crate::spec::BackupSpec;
use crate::{Error, Result};

// ---------------------------------------------------------------------------
// BackupClient
// ---------------------------------------------------------------------------

/// High-level backup management facade.
///
/// Owns resolved paths and provides convenience methods that compose
/// the lower-level modules (`backup`, `restore`, `schedule`, `doctor`)
/// into common workflows.
///
/// # Construction
///
/// - [`BackupClient::system`] -- production defaults with XDG paths.
/// - [`BackupClient::with_paths`] -- custom paths.
pub struct BackupClient {
    /// Resolved paths for backup data and configuration.
    paths: BackupPaths,
    /// Whether to run in dry-run mode.
    dry_run: bool,
}

impl BackupClient {
    // -----------------------------------------------------------------------
    // Constructors
    // -----------------------------------------------------------------------

    /// Create a `BackupClient` with production defaults.
    ///
    /// Resolves paths using XDG conventions.
    ///
    /// # Errors
    ///
    /// Returns an error if XDG directories cannot be determined.
    pub fn system() -> Result<Self> {
        let paths = BackupPaths::resolve()?;
        Ok(Self {
            paths,
            dry_run: false,
        })
    }

    /// Create a `BackupClient` with explicit paths.
    pub fn with_paths(paths: BackupPaths) -> Self {
        Self {
            paths,
            dry_run: false,
        }
    }

    /// Set dry-run mode.
    ///
    /// When enabled, backup operations are logged but not executed.
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    // -----------------------------------------------------------------------
    // Backup operations
    // -----------------------------------------------------------------------

    /// Run a backup according to the given specification.
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails or the backup command fails.
    pub fn backup(&self, spec: &BackupSpec) -> Result<BackupReport> {
        if self.dry_run {
            tracing::info!(
                name = %spec.name,
                "dry run: would run backup"
            );
            return Ok(BackupReport {
                name: spec.name.clone(),
                last_run: None,
                status: crate::report::BackupStatus::Ok,
                snapshot_count: 0,
                repo_size_bytes: 0,
                integrity: crate::report::IntegrityStatus::NotChecked,
                last_message: Some("dry run".into()),
            });
        }
        backup::run_backup(spec)
    }

    /// Run retention pruning according to the given specification.
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails or the prune command fails.
    pub fn prune(&self, spec: &BackupSpec) -> Result<PruneReport> {
        if self.dry_run {
            tracing::info!(
                name = %spec.name,
                "dry run: would run prune"
            );
            return Ok(PruneReport::empty());
        }
        backup::run_prune(spec)
    }

    // -----------------------------------------------------------------------
    // Restore operations
    // -----------------------------------------------------------------------

    /// Restore from a backup specification.
    ///
    /// # Errors
    ///
    /// Returns an error if the restore operation fails.
    pub fn restore(
        &self,
        spec: &BackupSpec,
        options: &RestoreOptions,
    ) -> Result<crate::report::RestoreReport> {
        RestoreManager::restore(spec, options)
    }

    /// Run a test restore to verify backup integrity.
    ///
    /// # Errors
    ///
    /// Returns an error if the test restore fails.
    pub fn test_restore(
        &self,
        spec: &BackupSpec,
    ) -> Result<crate::report::RestoreReport> {
        RestoreManager::test_restore(spec)
    }

    // -----------------------------------------------------------------------
    // Schedule operations
    // -----------------------------------------------------------------------

    /// Install a schedule for a backup job.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ScheduleError`] if installation fails.
    pub fn install_schedule(&self, spec: &BackupSpec) -> Result<()> {
        let mgr = ScheduleManager::new();
        mgr.install(&spec.name, &spec.schedule)
    }

    /// Remove a schedule for a backup job.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ScheduleError`] if removal fails.
    pub fn remove_schedule(&self, name: &str) -> Result<()> {
        let mgr = ScheduleManager::new();
        mgr.remove(name)
    }

    // -----------------------------------------------------------------------
    // Doctor
    // -----------------------------------------------------------------------

    /// Run diagnostic checks and return a report.
    ///
    /// # Errors
    ///
    /// Returns an error only for fundamental failures.
    #[cfg(feature = "doctor")]
    pub fn doctor(
        &self,
        scope: crate::doctor::DoctorScope,
    ) -> Result<crate::doctor::DoctorReport> {
        let doc = crate::doctor::Doctor::new();
        doc.run(&scope)
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Returns a reference to the resolved paths.
    pub fn paths(&self) -> &BackupPaths {
        &self.paths
    }

    /// Returns whether dry-run mode is active.
    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}
