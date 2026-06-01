//! Tailnet network topology and peer discovery.
//!
//! Provides types and operations for inspecting the tailnet topology,
//! listing peers, and querying the network layout.

use crate::report::PeerInfo;
use crate::Result;

// ---------------------------------------------------------------------------
// TailnetTopology
// ---------------------------------------------------------------------------

/// Represents the network topology of a tailnet.
///
/// Contains information about all peers in the tailnet, their connection
/// status, and the DERP relay configuration.
///
/// # Example
///
/// ```ignore
/// use toride_tailscale::tailnet::TailnetTopology;
/// use toride_tailscale::api::TailscaleApi;
///
/// let api = TailscaleApi::new();
/// let topology = TailnetTopology::from_api(&api).await?;
/// for peer in topology.peers() {
///     println!("{}: {} (online={})", peer.name, peer.ip_addresses.join(", "), peer.online);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct TailnetTopology {
    /// The name of this tailnet.
    tailnet_name: String,

    /// The current node's name.
    self_name: String,

    /// All peers in the tailnet.
    peers: Vec<PeerInfo>,
}

impl TailnetTopology {
    /// Create a new `TailnetTopology`.
    pub fn new(tailnet_name: String, self_name: String, peers: Vec<PeerInfo>) -> Self {
        Self {
            tailnet_name,
            self_name,
            peers,
        }
    }

    /// Fetch the tailnet topology from the local API.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or the response cannot
    /// be parsed.
    pub async fn from_api(api: &crate::api::TailscaleApi) -> Result<Self> {
        let status = api.get_status().await?;

        let tailnet_name = status
            .get("CurrentTailnet")
            .and_then(|t| t.get("Name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_owned();

        let self_name = status
            .get("Self")
            .and_then(|s| s.get("HostName"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_owned();

        // TODO: Parse peer list from the status response.
        let peers = Vec::new();

        Ok(Self {
            tailnet_name,
            self_name,
            peers,
        })
    }

    /// Returns the tailnet name.
    pub fn tailnet_name(&self) -> &str {
        &self.tailnet_name
    }

    /// Returns the current node's name.
    pub fn self_name(&self) -> &str {
        &self.self_name
    }

    /// Returns the list of all peers in the tailnet.
    pub fn peers(&self) -> &[PeerInfo] {
        &self.peers
    }

    /// Returns only the online (reachable) peers.
    pub fn online_peers(&self) -> Vec<&PeerInfo> {
        self.peers.iter().filter(|p| p.online).collect()
    }

    /// Returns only the exit nodes.
    pub fn exit_nodes(&self) -> Vec<&PeerInfo> {
        self.peers.iter().filter(|p| p.exit_node).collect()
    }
}
