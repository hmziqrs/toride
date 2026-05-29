use std::path::{Path, PathBuf};

use crate::Result;

/// Resolved paths for the `~/.ssh` directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshPaths {
    ssh_dir: PathBuf,
    config_path: PathBuf,
    known_hosts_path: PathBuf,
    authorized_keys_path: PathBuf,
}

impl Default for SshPaths {
    fn default() -> Self {
        Self::new().expect("home directory not found")
    }
}

impl SshPaths {
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
