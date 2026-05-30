use std::path::{Path, PathBuf};

use crate::Result;

/// Resolved paths for the `~/.ssh` directory.
///
/// All paths are resolved relative to the user's home directory at construction
/// time. The `Default` impl returns a best-effort set of paths, falling back to
/// `~/.ssh` when the home directory cannot be resolved (e.g., in containers).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshPaths {
    ssh_dir: PathBuf,
    config_path: PathBuf,
    known_hosts_path: PathBuf,
    authorized_keys_path: PathBuf,
}

impl Default for SshPaths {
    /// Return best-effort paths, falling back to `~/.ssh` if home is unavailable.
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        let ssh_dir = home.join(".ssh");
        Self {
            config_path: ssh_dir.join("config"),
            known_hosts_path: ssh_dir.join("known_hosts"),
            authorized_keys_path: ssh_dir.join("authorized_keys"),
            ssh_dir,
        }
    }
}

impl SshPaths {
    /// Create an `SshPaths` rooted at an arbitrary directory (for tests).
    #[cfg(test)]
    pub(crate) fn with_dir(dir: &std::path::Path) -> Self {
        Self {
            ssh_dir: dir.to_path_buf(),
            config_path: dir.join("config"),
            known_hosts_path: dir.join("known_hosts"),
            authorized_keys_path: dir.join("authorized_keys"),
        }
    }

    /// Resolve paths from the user's home directory.
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().ok_or(crate::Error::HomeNotFound)?;
        let ssh_dir = home.join(".ssh");
        let config_path = ssh_dir.join("config");
        let known_hosts_path = ssh_dir.join("known_hosts");
        let authorized_keys_path = ssh_dir.join("authorized_keys");
        Ok(Self {
            ssh_dir,
            config_path,
            known_hosts_path,
            authorized_keys_path,
        })
    }

    /// Path to `~/.ssh`.
    pub fn ssh_dir(&self) -> &Path {
        &self.ssh_dir
    }

    /// Path to `~/.ssh/config`.
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Path to `~/.ssh/known_hosts`.
    pub fn known_hosts_path(&self) -> &Path {
        &self.known_hosts_path
    }

    /// Path to `~/.ssh/authorized_keys`.
    pub fn authorized_keys_path(&self) -> &Path {
        &self.authorized_keys_path
    }

    /// Default key file name patterns to scan (without extension).
    pub fn default_key_names() -> &'static [&'static str] {
        &["id_rsa", "id_ecdsa", "id_ecdsa_sk", "id_ed25519", "id_ed25519_sk"]
    }
}
