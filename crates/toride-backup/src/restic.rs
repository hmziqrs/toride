//! Restic CLI wrapper.
//!
//! [`ResticClient`] provides a typed interface to the `restic` binary for
//! common backup operations: initialising repositories, creating snapshots,
//! listing snapshots, checking integrity, pruning, and restoring.
//!
//! All commands go through a centralised runner so that they are testable
//! and respect dry-run mode automatically.
//!
//! # Example
//!
//! ```ignore
//! use toride_backup::restic::ResticClient;
//!
//! let client = ResticClient::new("/mnt/backups/my-server")?;
//! client.init()?;
//! client.backup(&["/etc", "/home"])?;
//! let snapshots = client.snapshots()?;
//! ```

use std::path::{Path, PathBuf};

use crate::{Error, Result};

// ---------------------------------------------------------------------------
// ResticClient
// ---------------------------------------------------------------------------

/// Typed wrapper around the `restic` binary.
///
/// Every method constructs the appropriate argument list and delegates
/// execution to the underlying command runner. Arguments are always passed
/// as arrays -- no shell string concatenation is used.
pub struct ResticClient {
    /// Resolved path to the `restic` binary.
    binary: PathBuf,
    /// Repository path or URL.
    repo: PathBuf,
    /// Optional password command (e.g. `"cat /etc/restic/password"`).
    password_command: Option<String>,
    /// Extra environment variables.
    extra_env: Vec<(String, String)>,
}

impl ResticClient {
    /// Create a new restic client by locating `restic` on `$PATH`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BinaryNotFound`] if `restic` is not on `$PATH`.
    pub fn new(repo: impl AsRef<Path>) -> Result<Self> {
        let binary = which::which("restic").map_err(|_| {
            Error::BinaryNotFound("restic".into())
        })?;
        Ok(Self {
            binary,
            repo: repo.as_ref().to_path_buf(),
            password_command: None,
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
            password_command: None,
            extra_env: Vec::new(),
        }
    }

    /// Set the password command for repository authentication.
    #[must_use]
    pub fn with_password_command(mut self, cmd: impl Into<String>) -> Self {
        self.password_command = Some(cmd.into());
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

    /// Initialise a new restic repository.
    ///
    /// Runs `restic init --repo <repo>`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RepositoryInit`] if the init command fails.
    pub fn init(&self) -> Result<()> {
        // TODO: implement via Runner trait.
        tracing::info!(repo = %self.repo.display(), "restic init");
        Err(Error::RepositoryInit("not yet implemented".into()))
    }

    /// Check repository integrity.
    ///
    /// Runs `restic check --repo <repo>`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RepositoryAccess`] if the check command fails.
    pub fn check(&self) -> Result<String> {
        // TODO: implement via Runner trait.
        tracing::info!(repo = %self.repo.display(), "restic check");
        Err(Error::RepositoryAccess("not yet implemented".into()))
    }

    // -----------------------------------------------------------------------
    // Snapshot operations
    // -----------------------------------------------------------------------

    /// Create a backup snapshot of the given paths.
    ///
    /// Runs `restic backup --repo <repo> <paths...>`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the backup command fails.
    pub fn backup(&self, paths: &[&Path]) -> Result<String> {
        // TODO: implement via Runner trait.
        tracing::info!(
            repo = %self.repo.display(),
            paths = ?paths.iter().map(|p| p.display()).collect::<Vec<_>>(),
            "restic backup"
        );
        let _ = &self.extra_env;
        let _ = &self.password_command;
        Err(Error::CommandFailed("not yet implemented".into()))
    }

    /// List all snapshots in the repository.
    ///
    /// Runs `restic snapshots --repo <repo> --json`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the command fails.
    pub fn snapshots(&self) -> Result<String> {
        // TODO: implement via Runner trait.
        tracing::info!(repo = %self.repo.display(), "restic snapshots");
        Err(Error::CommandFailed("not yet implemented".into()))
    }

    /// Prune snapshots according to a retention policy.
    ///
    /// Runs `restic forget --repo <repo> --prune <keep-flags...>`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the prune command fails.
    pub fn prune(&self, keep_daily: Option<u32>, keep_weekly: Option<u32>, keep_monthly: Option<u32>) -> Result<String> {
        // TODO: implement via Runner trait.
        tracing::info!(
            repo = %self.repo.display(),
            keep_daily = ?keep_daily,
            keep_weekly = ?keep_weekly,
            keep_monthly = ?keep_monthly,
            "restic forget --prune"
        );
        Err(Error::CommandFailed("not yet implemented".into()))
    }

    // -----------------------------------------------------------------------
    // Restore
    // -----------------------------------------------------------------------

    /// Restore a snapshot to a target directory.
    ///
    /// Runs `restic restore <snapshot> --repo <repo> --target <target>`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RestoreFailed`] if the restore command fails.
    pub fn restore(&self, snapshot: &str, target: &Path) -> Result<()> {
        // TODO: implement via Runner trait.
        tracing::info!(
            repo = %self.repo.display(),
            snapshot = %snapshot,
            target = %target.display(),
            "restic restore"
        );
        Err(Error::RestoreFailed("not yet implemented".into()))
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Build the common restic arguments for any command.
    fn base_args(&self) -> Vec<String> {
        let mut args = vec![
            "--repo".to_string(),
            self.repo.to_string_lossy().to_string(),
        ];
        if let Some(ref pw_cmd) = self.password_command {
            args.push("--password-command".to_string());
            args.push(pw_cmd.clone());
        }
        args
    }
}
