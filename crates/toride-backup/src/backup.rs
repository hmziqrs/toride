//! High-level backup operations.
//!
//! Provides the [`run_backup`] and [`run_prune`] functions that coordinate
//! the full backup lifecycle: validate the spec, create the snapshot, apply
//! retention, and produce a typed report.

use crate::report::{BackupReport, BackupStatus, PruneReport};
use crate::spec::BackupSpec;
use crate::{Error, Result};

// ---------------------------------------------------------------------------
// Backup operations
// ---------------------------------------------------------------------------

/// Run a full backup according to the given specification.
///
/// Workflow:
/// 1. Validate the spec via [`BackupSpec::validate`].
/// 2. Delegate to the appropriate backend (restic or borg).
/// 3. Produce a [`BackupReport`] summarising the result.
///
/// # Errors
///
/// Returns [`Error`] if validation fails or the backup command fails.
pub fn run_backup(spec: &BackupSpec) -> Result<BackupReport> {
    spec.validate()?;

    tracing::info!(
        name = %spec.name,
        backend = %spec.backend,
        repo = %spec.repository.display(),
        "starting backup"
    );

    match spec.backend {
        crate::spec::Backend::Restic => run_restic_backup(spec),
        crate::spec::Backend::Borg => run_borg_backup(spec),
    }
}

/// Run retention pruning according to the given specification.
///
/// Workflow:
/// 1. Validate the spec.
/// 2. Delegate to the appropriate backend's prune command.
/// 3. Produce a [`PruneReport`] summarising the result.
///
/// # Errors
///
/// Returns [`Error`] if validation fails or the prune command fails.
pub fn run_prune(spec: &BackupSpec) -> Result<PruneReport> {
    spec.validate()?;

    tracing::info!(
        name = %spec.name,
        backend = %spec.backend,
        "starting prune"
    );

    match spec.backend {
        crate::spec::Backend::Restic => run_restic_prune(spec),
        crate::spec::Backend::Borg => run_borg_prune(spec),
    }
}

// ---------------------------------------------------------------------------
// Backend-specific implementations
// ---------------------------------------------------------------------------

fn run_restic_backup(spec: &BackupSpec) -> Result<BackupReport> {
    // TODO: create ResticClient and run backup.
    tracing::info!(name = %spec.name, "restic backup (not yet implemented)");

    let source_paths: Vec<&std::path::Path> = spec.sources.iter().map(std::path::PathBuf::as_path).collect();
    let _ = &source_paths;

    Ok(BackupReport {
        name: spec.name.clone(),
        last_run: Some(std::time::SystemTime::now()),
        status: BackupStatus::Ok,
        snapshot_count: 0,
        repo_size_bytes: 0,
        integrity: crate::report::IntegrityStatus::NotChecked,
        last_message: Some("backup not yet implemented".into()),
    })
}

fn run_borg_backup(spec: &BackupSpec) -> Result<BackupReport> {
    // TODO: create BorgClient and run backup.
    tracing::info!(name = %spec.name, "borg create (not yet implemented)");

    Ok(BackupReport {
        name: spec.name.clone(),
        last_run: Some(std::time::SystemTime::now()),
        status: BackupStatus::Ok,
        snapshot_count: 0,
        repo_size_bytes: 0,
        integrity: crate::report::IntegrityStatus::NotChecked,
        last_message: Some("backup not yet implemented".into()),
    })
}

fn run_restic_prune(spec: &BackupSpec) -> Result<PruneReport> {
    // TODO: create ResticClient and run prune.
    tracing::info!(name = %spec.name, "restic prune (not yet implemented)");

    Ok(PruneReport::empty())
}

fn run_borg_prune(spec: &BackupSpec) -> Result<PruneReport> {
    // TODO: create BorgClient and run prune.
    tracing::info!(name = %spec.name, "borg prune (not yet implemented)");

    Ok(PruneReport::empty())
}
