//! Borg Backup CLI wrapper.
//!
//! [`BorgClient`] provides a typed interface to the `borg` binary for
//! common backup operations: initialising repositories, creating archives,
//! listing archives, checking integrity, pruning, and extracting.
//!
//! All commands go through a centralised runner so that they are testable
//! and respect dry-run mode automatically.
//!
//! # Example
//!
//! ```ignore
//! use toride_backup::borg::BorgClient;
//!
//! let client = BorgClient::new("/mnt/backups/my-server")?;
//! client.init()?;
//! client.create("daily", &["/etc", "/home"])?;
//! let archives = client.list()?;
//! ```

use std::path::{Path, PathBuf};

use crate::{Error, Result};

// ---------------------------------------------------------------------------
// BorgClient
// ---------------------------------------------------------------------------

/// Typed wrapper around the `borg` binary.
///
/// Every method constructs the appropriate argument list and delegates
/// execution to the underlying command runner. Arguments are always passed
/// as arrays -- no shell string concatenation is used.
pub struct BorgClient {
    /// Resolved path to the `borg` binary.
    binary: PathBuf,
    /// Repository path or URL.
    repo: PathBuf,
    /// Optional passphrase or command to retrieve it.
    passphrase: Option<String>,
    /// Extra environment variables.
    extra_env: Vec<(String, String)>,
}

impl BorgClient {
    /// Create a new borg client by locating `borg` on `$PATH`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BinaryNotFound`] if `borg` is not on `$PATH`.
    pub fn new(repo: impl AsRef<Path>) -> Result<Self> {
        let binary = which::which("borg").map_err(|_| {
            Error::BinaryNotFound("borg".into())
        })?;
        Ok(Self {
            binary,
            repo: repo.as_ref().to_path_buf(),
            passphrase: None,
            extra_env: Vec::new(),
        })
    }

    /// Create a client with an explicit binary path.
    pub fn with_binary(
        binary: PathBuf,
        repo: impl AsRef<Path>,
    ) -> Self {
        Self {
            binary,
            repo: repo.as_ref().to_path_buf(),
            passphrase: None,
            extra_env: Vec::new(),
        }
    }

    /// Set the passphrase for repository encryption.
    #[must_use]
    pub fn with_passphrase(mut self, passphrase: impl Into<String>) -> Self {
        self.passphrase = Some(passphrase.into());
        self
    }

    /// Add an extra environment variable.
    #[must_use]
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_env.push((key.into(), value.into()));
        self
    }

    // -----------------------------------------------------------------------
    // Repository management
    // -----------------------------------------------------------------------

    /// Initialise a new borg repository.
    ///
    /// Runs `borg init --encryption=repokey <repo>`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RepositoryInit`] if the init command fails.
    pub fn init(&self) -> Result<()> {
        // TODO: implement via Runner trait.
        tracing::info!(repo = %self.repo.display(), "borg init");
        Err(Error::RepositoryInit("not yet implemented".into()))
    }

    /// Check repository integrity.
    ///
    /// Runs `borg check <repo>`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RepositoryAccess`] if the check command fails.
    pub fn check(&self) -> Result<String> {
        // TODO: implement via Runner trait.
        tracing::info!(repo = %self.repo.display(), "borg check");
        Err(Error::RepositoryAccess("not yet implemented".into()))
    }

    // -----------------------------------------------------------------------
    // Archive operations
    // -----------------------------------------------------------------------

    /// Create a new archive in the repository.
    ///
    /// Runs `borg create <repo>::<archive> <paths...>`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the create command fails.
    pub fn create(&self, archive: &str, paths: &[&Path]) -> Result<String> {
        // TODO: implement via Runner trait.
        tracing::info!(
            repo = %self.repo.display(),
            archive = %archive,
            paths = ?paths.iter().map(|p| p.display()).collect::<Vec<_>>(),
            "borg create"
        );
        let _ = &self.extra_env;
        let _ = &self.passphrase;
        Err(Error::CommandFailed("not yet implemented".into()))
    }

    /// List all archives in the repository.
    ///
    /// Runs `borg list <repo>`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the list command fails.
    pub fn list(&self) -> Result<String> {
        // TODO: implement via Runner trait.
        tracing::info!(repo = %self.repo.display(), "borg list");
        Err(Error::CommandFailed("not yet implemented".into()))
    }

    /// Prune archives according to a retention policy.
    ///
    /// Runs `borg prune <repo> --keep-daily=N --keep-weekly=N ...`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the prune command fails.
    pub fn prune(
        &self,
        keep_daily: Option<u32>,
        keep_weekly: Option<u32>,
        keep_monthly: Option<u32>,
    ) -> Result<String> {
        // TODO: implement via Runner trait.
        tracing::info!(
            repo = %self.repo.display(),
            keep_daily = ?keep_daily,
            keep_weekly = ?keep_weekly,
            keep_monthly = ?keep_monthly,
            "borg prune"
        );
        Err(Error::CommandFailed("not yet implemented".into()))
    }

    // -----------------------------------------------------------------------
    // Extract / restore
    // -----------------------------------------------------------------------

    /// Extract (restore) an archive to a target directory.
    ///
    /// Runs `borg extract <repo>::<archive> --destination <target>`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RestoreFailed`] if the extract command fails.
    pub fn extract(&self, archive: &str, target: &Path) -> Result<()> {
        // TODO: implement via Runner trait.
        tracing::info!(
            repo = %self.repo.display(),
            archive = %archive,
            target = %target.display(),
            "borg extract"
        );
        Err(Error::RestoreFailed("not yet implemented".into()))
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Build the repository path argument.
    fn repo_arg(&self) -> String {
        self.repo.to_string_lossy().to_string()
    }
}
