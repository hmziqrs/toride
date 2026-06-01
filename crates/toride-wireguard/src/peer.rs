//! Peer management types and operations.
//!
//! Provides types for managing the lifecycle of WireGuard peers: adding,
//! removing, and updating peers on a running interface.

use crate::error::Result;
use crate::spec::PeerSpec;

// ---------------------------------------------------------------------------
// PeerChange
// ---------------------------------------------------------------------------

/// A change to apply to a peer's configuration.
#[derive(Debug, Clone)]
pub enum PeerChange {
    /// Update the allowed-ips list.
    AllowedIps(Vec<String>),
    /// Update the endpoint address.
    Endpoint(String),
    /// Update the persistent keepalive interval.
    PersistentKeepalive(u32),
    /// Remove the persistent keepalive setting.
    RemoveKeepalive,
}

// ---------------------------------------------------------------------------
// PeerManager
// ---------------------------------------------------------------------------

/// Manages peer operations on a WireGuard interface.
///
/// This is a lightweight struct that carries the interface name and delegates
/// to the `wg` CLI for runtime peer changes.
#[derive(Debug)]
pub struct PeerManager {
    interface: String,
}

impl PeerManager {
    /// Create a new peer manager for the given interface.
    pub fn new(interface: &str) -> Self {
        Self {
            interface: interface.to_owned(),
        }
    }

    /// Returns the interface name.
    pub fn interface(&self) -> &str {
        &self.interface
    }

    /// Add a peer to the running interface.
    ///
    /// Executes `wg set <interface> peer <public_key> allowed-ips <ips>`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the `wg set` command fails.
    pub fn add_peer(&self, peer: &PeerSpec) -> Result<()> {
        tracing::info!(
            "adding peer {} to interface {}",
            peer.public_key,
            self.interface
        );
        // TODO: implement via `wg set`.
        Ok(())
    }

    /// Remove a peer from the running interface.
    ///
    /// Executes `wg set <interface> peer <public_key> remove`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::PeerNotFound`] if the peer does not exist,
    /// or [`Error::CommandFailed`] if the `wg set` command fails.
    pub fn remove_peer(&self, public_key: &str) -> Result<()> {
        tracing::info!(
            "removing peer {} from interface {}",
            public_key,
            self.interface
        );
        // TODO: implement via `wg set <if> peer <key> remove`.
        Ok(())
    }

    /// Apply a change to an existing peer.
    ///
    /// # Errors
    ///
    /// Returns [`Error::PeerNotFound`] if the peer does not exist,
    /// or [`Error::CommandFailed`] if the `wg set` command fails.
    pub fn update_peer(&self, public_key: &str, change: &PeerChange) -> Result<()> {
        tracing::info!(
            "updating peer {} on interface {}",
            public_key,
            self.interface
        );
        // TODO: implement via `wg set`.
        Ok(())
    }

    /// List all peers on the interface.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if `wg show` fails.
    pub fn list_peers(&self) -> Result<Vec<PeerSpec>> {
        tracing::debug!("listing peers on interface {}", self.interface);
        // TODO: implement via `wg show <interface> peers`.
        Ok(Vec::new())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_manager_new() {
        let mgr = PeerManager::new("wg0");
        assert_eq!(mgr.interface(), "wg0");
    }

    #[test]
    fn peer_change_variants() {
        let _change = PeerChange::AllowedIps(vec!["10.0.0.2/32".to_owned()]);
        let _change = PeerChange::Endpoint("1.2.3.4:51820".to_owned());
        let _change = PeerChange::PersistentKeepalive(25);
        let _change = PeerChange::RemoveKeepalive;
    }
}
