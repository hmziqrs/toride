//! Unified cloud client facade.
//!
//! [`CloudClient`] is the main entry point for cloud provider operations.
//! It detects the current provider and delegates to the appropriate
//! provider-specific client module.

use crate::aws::AwsClient;
use crate::digitalocean::DigitalOceanClient;
use crate::error::{Error, Result};
use crate::gcp::GcpClient;
use crate::hetzner::HetznerClient;
use crate::report::CloudReport;
use crate::spec::SecurityGroup;
use crate::CloudProvider;

// ---------------------------------------------------------------------------
// CloudClient
// ---------------------------------------------------------------------------

/// Unified cloud client that delegates to provider-specific implementations.
///
/// # Example
///
/// ```ignore
/// use toride_cloud::client::CloudClient;
///
/// let client = CloudClient::detect()?;
/// let groups = client.list_security_groups()?;
/// for group in &groups {
///     println!("{}: {} rules", group.name, group.rules.len());
/// }
/// ```
pub struct CloudClient {
    /// The detected cloud provider.
    pub provider: CloudProvider,
}

impl CloudClient {
    /// Create a client by auto-detecting the cloud provider.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ProviderNotFound`] if no provider can be detected.
    pub fn detect() -> Result<Self> {
        let provider = crate::detect::detect_provider()?;
        Ok(Self { provider })
    }

    /// Create a client for a specific provider.
    #[must_use]
    pub fn for_provider(provider: CloudProvider) -> Self {
        Self { provider }
    }

    /// List all security groups for the detected provider.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the provider CLI fails.
    pub fn list_security_groups(&self) -> Result<Vec<SecurityGroup>> {
        match self.provider {
            CloudProvider::Aws => {
                let client = AwsClient::new("us-east-1");
                client.list_security_groups()
            }
            CloudProvider::Gcp => {
                let client = GcpClient::new("default");
                client.list_firewall_rules()
            }
            CloudProvider::DigitalOcean => {
                let client = DigitalOceanClient::new();
                client.list_firewalls()
            }
            CloudProvider::Hetzner => {
                let client = HetznerClient::new();
                client.list_firewalls()
            }
            CloudProvider::Unknown => {
                Err(Error::ProviderNotFound(
                    "cannot list security groups for unknown provider".to_string(),
                ))
            }
        }
    }

    /// Generate a full cloud report for the detected provider.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the provider CLI fails.
    pub fn report(&self) -> Result<CloudReport> {
        let mut report = CloudReport::new(self.provider);

        match self.list_security_groups() {
            Ok(groups) => {
                report.security_groups = groups;
            }
            Err(e) => {
                report.push(
                    crate::report::Finding::new(
                        "client.list-failed",
                        crate::report::Severity::Error,
                        "Failed to list security groups",
                    )
                    .detail(format!("{e}")),
                );
            }
        }

        Ok(report)
    }
}
