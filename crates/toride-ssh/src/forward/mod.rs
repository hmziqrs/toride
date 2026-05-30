pub mod control;

use std::collections::HashMap;
use std::path::Path;

use crate::paths::SshPaths;
use crate::{Error, Result};

pub use control::{ControlSession, ForwardType, PortForward};

/// Port forwarding management via ControlMaster sessions.
///
/// Obtained from [`SshManager::forward()`](crate::SshManager::forward).
pub struct ForwardService<'a> {
    paths: &'a SshPaths,
}

impl<'a> ForwardService<'a> {
    pub(crate) fn new(paths: &'a SshPaths) -> Self {
        Self { paths }
    }

    /// List all active port forwards across all discovered ControlMaster sessions.
    ///
    /// Returns a list of `(session, forwards)` pairs.  Sessions whose
    /// forwards cannot be listed are included with an empty forward list
    /// (the error is logged but not propagated).
    ///
    /// # Errors
    ///
    /// Returns [`Error::TaskFailed`] if the background task for discovering
    /// ControlMaster sessions panics or is cancelled.
    pub async fn list(&self) -> Result<Vec<(ControlSession, Vec<PortForward>)>> {
        let sessions = self.list_sessions().await?;
        let mut results = Vec::with_capacity(sessions.len());

        for session in sessions {
            match control::list_forwards(&session.control_path).await {
                Ok(forwards) => results.push((session, forwards)),
                Err(e) => {
                    tracing::warn!(
                        "failed to list forwards for {}: {e}",
                        session.control_path.display()
                    );
                    results.push((session, Vec::new()));
                }
            }
        }

        Ok(results)
    }

    /// Discover active ControlMaster sessions.
    ///
    /// Scans `~/.ssh/cm-*`, `~/.ssh/control-*`, `~/.ssh/mux-*`,
    /// `~/.ssh/ctrl-*`, and `/tmp/ssh-*` for control sockets.  Each
    /// candidate is verified with `ssh -O check` before inclusion.
    ///
    /// # Errors
    ///
    /// Returns [`Error::TaskFailed`] if the background scan task panics
    /// or is cancelled.
    pub async fn list_sessions(&self) -> Result<Vec<ControlSession>> {
        control::list_sessions(self.paths.ssh_dir()).await
    }

    /// Cancel a port forward on a specific session by local port number.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ForwardNotFound`] if no forward exists on the
    /// given local port, [`Error::ForwardFailed`] if the control path is
    /// not valid UTF-8, or [`Error::CommandFailed`] if the cancel command
    /// fails.
    pub async fn cancel(&self, control_path: &Path, local_port: u16) -> Result<()> {
        control::cancel_forward(control_path, local_port).await
    }

    /// List forwards for a single session identified by its control socket path.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ForwardFailed`] if the control path is not valid
    /// UTF-8, or [`Error::CommandFailed`] if the `ssh -O list` command fails.
    pub async fn list_forwards(&self, control_path: &Path) -> Result<Vec<PortForward>> {
        control::list_forwards(control_path).await
    }

    /// Cancel a known forward (avoids the extra list round-trip).
    ///
    /// # Errors
    ///
    /// Returns [`Error::ForwardFailed`] if the control path is not valid
    /// UTF-8, or [`Error::CommandFailed`] if the `ssh -O cancel` command fails.
    pub async fn cancel_known(&self, control_path: &Path, forward: &PortForward) -> Result<()> {
        control::cancel_known_forward(control_path, forward).await
    }

    /// Gracefully shut down a ControlMaster session.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ForwardFailed`] if the control path is not valid
    /// UTF-8, or [`Error::CommandFailed`] if the `ssh -O exit` command
    /// fails for a reason other than a stale socket.
    pub async fn exit_session(&self, control_path: &Path) -> Result<()> {
        control::exit_session(control_path).await
    }

    /// Detect duplicate local port bindings across all active sessions.
    ///
    /// When two different ControlMaster sessions both forward the same
    /// local port, only one can actually be listening.  This method
    /// returns a map from each conflicting port number to the list of
    /// control socket paths that claim it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::TaskFailed`] if listing sessions or forwards
    /// fails due to a background task panic.
    pub async fn conflicting_local_ports(
        &self,
    ) -> Result<HashMap<u16, Vec<std::path::PathBuf>>> {
        let sessions = self.list_sessions().await?;
        let mut port_owners: HashMap<u16, Vec<std::path::PathBuf>> = HashMap::new();

        for session in sessions {
            match control::list_forwards(&session.control_path).await {
                Ok(forwards) => {
                    for fwd in forwards {
                        port_owners
                            .entry(fwd.local_port)
                            .or_default()
                            .push(session.control_path.clone());
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "failed to list forwards for {}: {e}",
                        session.control_path.display()
                    );
                }
            }
        }

        // Keep only ports claimed by more than one session.
        port_owners.retain(|_, owners| owners.len() > 1);
        Ok(port_owners)
    }

    /// Test connectivity to a locally forwarded port.
    ///
    /// Attempts a TCP connection to `127.0.0.1:<local_port>` with a short
    /// timeout (2 seconds).  Returns `Ok(())` if the connection succeeds,
    /// or an error describing the failure.
    ///
    /// This is useful for verifying that a port forward is actually
    /// forwarding traffic to the expected remote service.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ForwardFailed`] if the connection cannot be
    /// established (port not listening, connection refused, timeout).
    pub async fn test_connectivity(&self, local_port: u16) -> Result<()> {
        let addr = format!("127.0.0.1:{local_port}");

        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            tokio::net::TcpStream::connect(&addr),
        )
        .await
        .map_err(|_| {
            Error::ForwardFailed(format!(
                "connection to {addr} timed out after 2 seconds"
            ))
        })?
        .map_err(|e| {
            Error::ForwardFailed(format!("cannot connect to {addr}: {e}"))
        })?;

        tracing::debug!("successfully connected to forwarded port {local_port}");
        Ok(())
    }
}
