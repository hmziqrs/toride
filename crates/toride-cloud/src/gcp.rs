//! GCP firewall rule management.
//!
//! Provides typed wrappers around the `gcloud` CLI for managing GCP firewall
//! rules and VPC network configuration.

use crate::error::{Error, Result};
use crate::spec::{FirewallRule, SecurityGroup};
use crate::CloudProvider;

// ---------------------------------------------------------------------------
// GcpClient
// ---------------------------------------------------------------------------

/// Client for managing GCP firewall rules.
///
/// Delegates command execution to the `gcloud` CLI.
pub struct GcpClient {
    /// GCP project ID.
    pub project: String,
    /// GCP region or zone.
    pub region: Option<String>,
}

impl GcpClient {
    /// Create a new GCP client for the given project.
    #[must_use]
    pub fn new(project: impl Into<String>) -> Self {
        Self {
            project: project.into(),
            region: None,
        }
    }

    /// Set the GCP region.
    #[must_use]
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// List all firewall rules in the project.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the `gcloud` CLI is not installed
    /// or returns a non-zero exit code.
    pub fn list_firewall_rules(&self) -> Result<Vec<SecurityGroup>> {
        // TODO: Implement `gcloud compute firewall-rules list --format=json`.
        let _ = &self.project;
        let _ = &self.region;
        Ok(Vec::new())
    }

    /// Get a firewall rule by name.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ProviderNotFound`] if the rule does not exist.
    pub fn get_firewall_rule(&self, name: &str) -> Result<SecurityGroup> {
        let _ = name;
        // TODO: Implement `gcloud compute firewall-rules describe`.
        Err(Error::ProviderNotFound(format!(
            "firewall rule {name} not found"
        )))
    }

    /// Create a new firewall rule.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if creation fails.
    pub fn create_firewall_rule(
        &self,
        name: &str,
        network: &str,
        rules: &[FirewallRule],
    ) -> Result<SecurityGroup> {
        let _ = (name, network, rules);
        // TODO: Implement `gcloud compute firewall-rules create`.
        Ok(SecurityGroup::new(name, CloudProvider::Gcp))
    }

    /// Delete a firewall rule by name.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if deletion fails.
    pub fn delete_firewall_rule(&self, name: &str) -> Result<()> {
        let _ = name;
        // TODO: Implement `gcloud compute firewall-rules delete`.
        Ok(())
    }

    /// Update an existing firewall rule.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the update fails.
    pub fn update_firewall_rule(
        &self,
        name: &str,
        rules: &[FirewallRule],
    ) -> Result<()> {
        let _ = (name, rules);
        // TODO: Implement `gcloud compute firewall-rules update`.
        Ok(())
    }
}
