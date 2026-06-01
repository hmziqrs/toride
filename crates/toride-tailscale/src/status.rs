//! Node status and connection state.
//!
//! Provides [`NodeStatus`] for querying the current state of the Tailscale
//! daemon: whether it is connected, the node's identity, IP addresses, and
//! the active exit node.

use crate::report::{ConnectionStatus, TailscaleReport};
use crate::Result;

// ---------------------------------------------------------------------------
// NodeStatus
// ---------------------------------------------------------------------------

/// Current status of the local Tailscale node.
///
/// Wraps the raw API response into a structured type with typed accessors.
///
/// # Example
///
/// ```ignore
/// use toride_tailscale::status::NodeStatus;
/// use toride_tailscale::api::TailscaleApi;
///
/// let api = TailscaleApi::new();
/// let status = NodeStatus::from_api(&api).await?;
/// println!("Connected as {} ({})", status.name(), status.connection());
/// ```
#[derive(Debug, Clone)]
pub struct NodeStatus {
    /// The raw JSON status response from the local API.
    raw: serde_json::Value,
}

impl NodeStatus {
    /// Create a `NodeStatus` from a raw API response.
    pub fn from_raw(raw: serde_json::Value) -> Self {
        Self { raw }
    }

    /// Fetch the node status from the local API.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    pub async fn from_api(api: &crate::api::TailscaleApi) -> Result<Self> {
        let raw = api.get_status().await?;
        Ok(Self { raw })
    }

    /// Returns the node's hostname.
    pub fn name(&self) -> &str {
        self.raw
            .get("Self")
            .and_then(|s| s.get("HostName"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
    }

    /// Returns the node's Tailscale IP addresses.
    pub fn ip_addresses(&self) -> Vec<String> {
        let mut ips = Vec::new();

        if let Some(addrs) = self.raw.get("Self").and_then(|s| s.get("TailscaleIPs")) {
            if let Some(arr) = addrs.as_array() {
                for ip in arr {
                    if let Some(s) = ip.as_str() {
                        ips.push(s.to_owned());
                    }
                }
            }
        }

        ips
    }

    /// Returns the connection status.
    pub fn connection(&self) -> ConnectionStatus {
        match self
            .raw
            .get("BackendState")
            .and_then(|v| v.as_str())
            .unwrap_or("")
        {
            "Running" => ConnectionStatus::Connected,
            "NeedsLogin" | "Stopped" => ConnectionStatus::Disconnected,
            "Starting" => ConnectionStatus::Starting,
            _ => ConnectionStatus::Unknown,
        }
    }

    /// Returns the tailnet name.
    pub fn tailnet(&self) -> &str {
        self.raw
            .get("CurrentTailnet")
            .and_then(|t| t.get("Name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
    }

    /// Convert this status into a [`TailscaleReport`].
    pub fn to_report(&self) -> TailscaleReport {
        TailscaleReport {
            connected: self.connection() == ConnectionStatus::Connected,
            node_name: self.name().to_owned(),
            tailnet: self.tailnet().to_owned(),
            ip_addresses: self.ip_addresses(),
            exit_node: None, // TODO: parse from raw response
            dns_enabled: true, // TODO: parse from raw response
        }
    }

    /// Returns the raw JSON value for direct inspection.
    pub fn raw(&self) -> &serde_json::Value {
        &self.raw
    }
}
