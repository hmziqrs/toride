//! Restore workflows for backup repositories.
//!
//! Provides high-level restore operations that coordinate between the
//! restic/borg clients and produce typed [`RestoreReport`] values.
//! Supports full restores, partial restores (specific paths), and test
//! restores for integrity verification.

use std::path::Path;

use crate::report::RestoreReport;
use crate::spec::BackupSpec;
use crate::{Error, Result};

// ---------------------------------------------------------------------------
// RestoreOptions
// ---------------------------------------------------------------------------

/// Options for a restore operation.
#[derive(Debug, Clone)]
pub struct RestoreOptions {
    /// Snapshot or archive ID to restore from.
    /// If `None`, restores from the latest snapshot.
    pub snapshot_id: Option<String>,
    /// Specific paths to restore (empty = full restore).
    pub paths: Vec<String>,
    /// Target directory for the restore.
    pub target: String,
    /// Whether to verify the restore by comparing file checksums.
    pub verify: bool,
    /// Whether this is a test restore (restored to a temporary location).
    pub test: bool,
}

impl RestoreOptions {
    /// Create restore options targeting a specific directory.
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            snapshot_id: None,
            paths: Vec::new(),
            target: target.into(),
            verify: false,
            test: false,
        }
    }

    /// Restore from a specific snapshot.
    #[must_use]
    pub fn with_snapshot(mut self, id: impl Into<String>) -> Self {
        self.snapshot_id = Some(id.into());
        self
    }

    /// Restore only specific paths.
    #[must_use]
    pub fn with_paths(mut self, paths: Vec<String>) -> Self {
        self.paths = paths;
        self
    }

    /// Enable verification after restore.
    #[must_use]
    pub fn with_verify(mut self) -> Self {
        self.verify = true;
        self
    }

    /// Mark as a test restore.
    #[must_use]
    pub fn as_test(mut self) -> Self {
        self.test = true;
        self
    }
}

// ---------------------------------------------------------------------------
// RestoreManager
// ---------------------------------------------------------------------------

/// Manages restore workflows for backup repositories.
///
/// Coordinates between the backend-specific clients (restic/borg) and
/// provides high-level restore operations with reporting.
pub struct RestoreManager;

impl RestoreManager {
    /// Perform a full restore from the given backup spec.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RestoreFailed`] if the restore operation fails.
    pub fn restore(_spec: &BackupSpec, _options: &RestoreOptions) -> Result<RestoreReport> {
        // TODO: delegate to ResticClient or BorgClient based on spec.backend.
        tracing::info!("restore operation (not yet implemented)");
        Err(Error::RestoreFailed("not yet implemented".into()))
    }

    /// Perform a test restore to verify backup integrity.
    ///
    /// Restores to a temporary directory and optionally verifies file
    /// checksums, then cleans up the temporary data.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RestoreFailed`] if the test restore fails.
    pub fn test_restore(spec: &BackupSpec) -> Result<RestoreReport> {
        let target = format!("/tmp/toride-backup-test-{}", spec.name);
        let options = RestoreOptions::new(&target)
            .as_test()
            .with_verify();

        match Self::restore(spec, &options) {
            Ok(report) => {
                // TODO: clean up temporary directory.
                tracing::info!(
                    target = %target,
                    "test restore completed, cleaning up"
                );
                Ok(report)
            }
            Err(e) => {
                // TODO: clean up temporary directory even on failure.
                tracing::warn!(
                    target = %target,
                    error = %e,
                    "test restore failed, cleaning up"
                );
                Err(e)
            }
        }
    }

    /// Verify that a restore target matches the original backup source.
    ///
    /// Compares file counts and total sizes between source and restore.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RestoreFailed`] if verification fails.
    pub fn verify(_source: &Path, _restore_target: &Path) -> Result<bool> {
        // TODO: walk both trees and compare file checksums.
        tracing::info!("restore verification (not yet implemented)");
        Err(Error::RestoreFailed("verification not yet implemented".into()))
    }
}
