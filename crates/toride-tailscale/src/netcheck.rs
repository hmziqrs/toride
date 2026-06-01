//! Network connectivity and DERP relay checks.
//!
//! Provides [`NetcheckRunner`] for performing network connectivity checks
//! through the Tailscale local API. Reports UDP availability, IPv6 support,
//! DERP relay latency, and NAT traversal capabilities.

use crate::report::NetcheckReport;
use crate::Result;

// ---------------------------------------------------------------------------
// NetcheckRunner
// ---------------------------------------------------------------------------

/// Runs network connectivity checks via the Tailscale local API.
///
/// `NetcheckRunner` queries the local API for the latest netcheck results
/// and parses them into a structured [`NetcheckReport`].
///
/// # Example
///
/// ```ignore
/// use toride_tailscale::netcheck::NetcheckRunner;
/// use toride_tailscale::api::TailscaleApi;
///
/// let api = TailscaleApi::new();
/// let runner = NetcheckRunner::new(&api);
/// let report = runner.run().await?;
/// println!("UDP: {}, IPv6: {}", report.udp, report.ipv6);
/// ```
pub struct NetcheckRunner<'a> {
    /// Reference to the Tailscale API client.
    api: &'a crate::api::TailscaleApi,
}

impl<'a> NetcheckRunner<'a> {
    /// Create a new `NetcheckRunner` with the given API client.
    pub fn new(api: &'a crate::api::TailscaleApi) -> Self {
        Self { api }
    }

    /// Run a network connectivity check and return the report.
    ///
    /// Queries `GET /localapi/v0/netcheck` and parses the response into
    /// a structured [`NetcheckReport`].
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or the response cannot
    /// be parsed.
    pub async fn run(&self) -> Result<NetcheckReport> {
        let raw = self.api.get_netcheck().await?;

        let connectivity = raw
            .get("MappingVariesByDestIP")
            .map(|v| v.is_boolean())
            .unwrap_or(false);

        let derp_region = raw
            .get("PreferredDERP")
            .and_then(|v| v.as_i64())
            .map(|id| format!("DERP-{id}"));

        let udp = raw.get("UDP").and_then(|v| v.as_bool()).unwrap_or(false);
        let ipv6 = raw.get("IPv6").and_then(|v| v.as_bool()).unwrap_or(false);
        let hairpin = raw.get("HairPinning").and_then(|v| v.as_bool()).unwrap_or(false);

        // Parse DERP latency map.
        let mut derp_latency = Vec::new();
        if let Some(latency_map) = raw.get("DERPLatency").and_then(|v| v.as_object()) {
            for (region, latency) in latency_map {
                if let Some(ms) = latency.as_f64() {
                    derp_latency.push((region.clone(), ms));
                }
            }
        }
        derp_latency.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(NetcheckReport {
            connectivity,
            derp_region,
            derp_latency,
            udp,
            ipv6,
            hairpin,
            port_mapping: Vec::new(),
        })
    }
}
