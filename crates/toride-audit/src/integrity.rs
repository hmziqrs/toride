//! AIDE file integrity monitoring management.
//!
//! Provides high-level operations for managing AIDE (Advanced Intrusion
//! Detection Environment) including database initialization, integrity
//! checks, and report generation.

use crate::{AuditPaths, Error, Result};

// ---------------------------------------------------------------------------
// IntegrityStatus
// ---------------------------------------------------------------------------

/// Status of the AIDE integrity check.
#[derive(Debug, Clone)]
pub struct IntegrityStatus {
    /// Whether the AIDE database is initialized.
    pub database_initialized: bool,
    /// Number of files in the AIDE database.
    pub file_count: Option<usize>,
    /// Whether the last integrity check passed.
    pub last_check_passed: Option<bool>,
    /// Output from the last check.
    pub last_check_output: Option<String>,
}

// ---------------------------------------------------------------------------
// IntegrityManager
// ---------------------------------------------------------------------------

/// High-level manager for AIDE file integrity monitoring.
///
/// Provides methods for initializing the AIDE database, running integrity
/// checks, and managing the AIDE configuration.
pub struct IntegrityManager<'a> {
    runner: &'a dyn toride_runner::Runner,
    paths: &'a AuditPaths,
}

impl<'a> IntegrityManager<'a> {
    /// Create a new integrity manager with the given runner and paths.
    pub fn new(runner: &'a dyn toride_runner::Runner, paths: &'a AuditPaths) -> Self {
        Self { runner, paths }
    }

    /// Initialize a new AIDE database.
    ///
    /// Runs `aide --init` to create the reference database.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BinaryNotFound`] if `aide` is not available.
    /// Returns [`Error::CommandFailed`] if initialization fails.
    pub fn initialize(&self) -> Result<()> {
        let bin = which::which("aide")
            .map_err(|_| Error::BinaryNotFound("aide".to_owned()))?;
        self.runner.run_checked(bin, &["--init"])?;
        Ok(())
    }

    /// Run an integrity check against the AIDE database.
    ///
    /// Runs `aide --check` and returns the output.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BinaryNotFound`] if `aide` is not available.
    pub fn check(&self) -> Result<String> {
        let bin = which::which("aide")
            .map_err(|_| Error::BinaryNotFound("aide".to_owned()))?;
        let output = self.runner.run_output(bin, &["--check"])?;
        Ok(output)
    }

    /// Update the AIDE database after a check.
    ///
    /// Runs `aide --update` to update the reference database with
    /// legitimate changes.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BinaryNotFound`] if `aide` is not available.
    /// Returns [`Error::CommandFailed`] if the update fails.
    pub fn update(&self) -> Result<()> {
        let bin = which::which("aide")
            .map_err(|_| Error::BinaryNotFound("aide".to_owned()))?;
        self.runner.run_checked(bin, &["--update"])?;
        Ok(())
    }

    /// Check the integrity status of the AIDE subsystem.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the database cannot be checked.
    pub fn status(&self) -> Result<IntegrityStatus> {
        let db_path = self.paths.aide_db_dir.join("aide.db.gz");
        let initialized = db_path.exists();

        Ok(IntegrityStatus {
            database_initialized: initialized,
            file_count: None,
            last_check_passed: None,
            last_check_output: None,
        })
    }
}
