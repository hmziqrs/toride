//! SSH agent management (listing, adding, removing keys).

mod client;
mod session;

pub use session::ControlSession;

use std::path::Path;

use crate::paths::SshPaths;
use crate::Error;
use crate::Result;
use crate::SshKey;

/// SSH agent operations.
pub struct AgentService<'a> {
    paths: &'a SshPaths,
}

impl<'a> AgentService<'a> {
    pub(crate) fn new(paths: &'a SshPaths) -> Self {
        Self { paths }
    }

    /// Check if the SSH agent is reachable.
    ///
    /// Returns `true` when `SSH_AUTH_SOCK` points to an existing socket and
    /// we can successfully connect to the agent.
    pub async fn status(&self) -> Result<bool> {
        #[cfg(feature = "agent")]
        {
            // `connect()` already validates `SSH_AUTH_SOCK` and socket
            // existence, so no need to duplicate those checks here.
            match client::connect().await {
                Ok(mut c) => {
                    c.request_identities()
                        .await
                        .map_err(|e| Error::AgentOperationFailed(e.to_string()))?;
                    Ok(true)
                }
                Err(Error::AgentNotAvailable) => Ok(false),
                Err(e) => Err(e),
            }
        }

        #[cfg(not(feature = "agent"))]
        {
            match crate::runner::ssh_add_list().await {
                Ok(_) => Ok(true),
                Err(Error::CommandFailed(_)) => Ok(false),
                Err(e) => Err(e),
            }
        }
    }

    /// List all keys currently loaded in the SSH agent.
    pub async fn list_keys(&self) -> Result<Vec<SshKey>> {
        client::list_identities().await
    }

    /// Add a private key to the SSH agent.
    pub async fn add_key(&self, key_path: &Path) -> Result<()> {
        client::add_key(key_path).await
    }

    /// Remove a key from the SSH agent.
    pub async fn remove_key(&self, key_path: &Path) -> Result<()> {
        client::remove_key(key_path).await
    }

    /// Remove all keys from the SSH agent (`ssh-add -D`).
    ///
    /// Returns the number of keys removed (best-effort; some agents don't report count).
    pub async fn remove_all(&self) -> Result<()> {
        let result = tokio::task::spawn_blocking(|| {
            duct::cmd("ssh-add", ["-D"])
                .read()
                .map_err(|e| Error::CommandFailed(format!("ssh-add -D failed: {e}")))
        })
        .await
        .map_err(|e| Error::TaskFailed(e.to_string()))??;

        tracing::debug!("ssh-add -D output: {result}");
        Ok(())
    }

    /// Add a key to the SSH agent with a lifetime limit (`ssh-add -t`).
    ///
    /// `lifetime_seconds` specifies how long the key should remain loaded.
    pub async fn add_key_with_lifetime(
        &self,
        key_path: &Path,
        lifetime_seconds: u32,
    ) -> Result<()> {
        let path_str = key_path
            .to_str()
            .ok_or_else(|| Error::CommandFailed("key path is not valid UTF-8".to_owned()))?
            .to_owned();
        let lifetime = lifetime_seconds.to_string();

        tokio::task::spawn_blocking(move || {
            duct::cmd("ssh-add", ["-t", &lifetime, &path_str])
                .read()
                .map_err(|e| Error::CommandFailed(format!("ssh-add -t failed: {e}")))?;
            Ok(())
        })
        .await
        .map_err(|e| Error::TaskFailed(e.to_string()))?
    }

    /// Add a key to the SSH agent with confirmation required (`ssh-add -c`).
    ///
    /// The agent will request user confirmation each time the key is used.
    pub async fn add_key_with_confirmation(&self, key_path: &Path) -> Result<()> {
        let path_str = key_path
            .to_str()
            .ok_or_else(|| Error::CommandFailed("key path is not valid UTF-8".to_owned()))?
            .to_owned();

        tokio::task::spawn_blocking(move || {
            duct::cmd("ssh-add", ["-c", &path_str])
                .read()
                .map_err(|e| Error::CommandFailed(format!("ssh-add -c failed: {e}")))?;
            Ok(())
        })
        .await
        .map_err(|e| Error::TaskFailed(e.to_string()))?
    }

    /// List active ControlMaster sessions.
    ///
    /// Scans for control socket files in the SSH directory and `/tmp`,
    /// verifying each is still alive.
    pub async fn list_sessions(&self) -> Result<Vec<ControlSession>> {
        session::list_sessions(self.paths.ssh_dir()).await
    }
}
