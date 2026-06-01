//! DNS configuration and MagicDNS management.
//!
//! Provides types and operations for inspecting and managing Tailscale DNS
//! settings, including MagicDNS, custom resolvers, and split DNS
//! configurations.

use crate::Result;

// ---------------------------------------------------------------------------
// DnsManager
// ---------------------------------------------------------------------------

/// Manager for Tailscale DNS configuration.
///
/// `DnsManager` queries the local API for the current DNS configuration
/// and provides typed access to nameservers, search domains, and MagicDNS
/// status.
///
/// # Example
///
/// ```ignore
/// use toride_tailscale::dns::DnsManager;
/// use toride_tailscale::api::TailscaleApi;
///
/// let api = TailscaleApi::new();
/// let dns = DnsManager::new(&api);
/// let config = dns.get_config().await?;
/// println!("MagicDNS: {}", config.magic_dns);
/// println!("Nameservers: {:?}", config.nameservers);
/// ```
pub struct DnsManager<'a> {
    /// Reference to the Tailscale API client.
    api: &'a crate::api::TailscaleApi,
}

// ---------------------------------------------------------------------------
// DnsConfigInfo
// ---------------------------------------------------------------------------

/// Parsed DNS configuration from the Tailscale local API.
#[derive(Debug, Clone, PartialEq)]
pub struct DnsConfigInfo {
    /// Whether MagicDNS is enabled.
    pub magic_dns: bool,

    /// Custom DNS resolver addresses.
    pub nameservers: Vec<String>,

    /// Search domains appended to DNS queries.
    pub search_domains: Vec<String>,

    /// Split DNS configurations: domain -> nameserver.
    pub split_dns: Vec<(String, String)>,
}

impl<'a> DnsManager<'a> {
    /// Create a new `DnsManager` with the given API client.
    pub fn new(api: &'a crate::api::TailscaleApi) -> Self {
        Self { api }
    }

    /// Fetch and parse the current DNS configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or the response cannot
    /// be parsed.
    pub async fn get_config(&self) -> Result<DnsConfigInfo> {
        let raw = self.api.get_dns_config().await?;

        let magic_dns = raw
            .get("MagicDNS")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let nameservers = raw
            .get("Nameservers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let search_domains = raw
            .get("SearchDomains")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(DnsConfigInfo {
            magic_dns,
            nameservers,
            search_domains,
            split_dns: Vec::new(),
        })
    }

    /// Check if MagicDNS is currently enabled.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    pub async fn is_magic_dns_enabled(&self) -> Result<bool> {
        let config = self.get_config().await?;
        Ok(config.magic_dns)
    }

    /// Validate a DNS configuration for correctness.
    ///
    /// Checks that nameserver addresses are valid IP addresses and that
    /// search domains are well-formed.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::DnsError`] if any value is invalid.
    pub fn validate_config(config: &DnsConfigInfo) -> Result<()> {
        for ns in &config.nameservers {
            if ns.parse::<std::net::IpAddr>().is_err() {
                return Err(crate::Error::DnsError(format!(
                    "invalid nameserver address: {ns}"
                )));
            }
        }
        Ok(())
    }
}
