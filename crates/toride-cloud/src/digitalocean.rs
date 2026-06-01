//! DigitalOcean firewall management.
//!
//! Provides typed wrappers around the `doctl` CLI for managing DigitalOcean
//! cloud firewalls and their rules.

use crate::error::{Error, Result};
use crate::spec::{FirewallRule, SecurityGroup};
use crate::CloudProvider;

// ---------------------------------------------------------------------------
// DigitalOceanClient
// ---------------------------------------------------------------------------

/// Client for managing DigitalOcean firewalls.
///
/// Delegates command execution to the `doctl` CLI.
pub struct DigitalOceanClient {
    /// DigitalOcean access token (uses `doctl` config if `None`).
    pub access_token: Option<String>,
}

impl DigitalOceanClient {
    /// Create a new DigitalOcean client.
    #[must_use]
    pub fn new() -> Self {
        Self { access_token: None }
    }

    /// Set the access token explicitly.
    #[must_use]
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.access_token = Some(token.into());
        self
    }

    /// List all firewalls in the account.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the `doctl` CLI is not installed
    /// or returns a non-zero exit code.
    pub fn list_firewalls(&self) -> Result<Vec<SecurityGroup>> {
        // TODO: Implement `doctl compute firewall list --format json`.
        let _ = &self.access_token;
        Ok(Vec::new())
    }

    /// Get a firewall by ID.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ProviderNotFound`] if the firewall does not exist.
    pub fn get_firewall(&self, firewall_id: &str) -> Result<SecurityGroup> {
        let _ = firewall_id;
        // TODO: Implement `doctl compute firewall get`.
        Err(Error::ProviderNotFound(format!(
            "firewall {firewall_id} not found"
        )))
    }

    /// Create a new firewall.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if creation fails.
    pub fn create_firewall(
        &self,
        name: &str,
        inbound_rules: &[FirewallRule],
        outbound_rules: &[FirewallRule],
    ) -> Result<SecurityGroup> {
        let _ = (name, inbound_rules, outbound_rules);
        // TODO: Implement `doctl compute firewall create`.
        Ok(SecurityGroup::new(name, CloudProvider::DigitalOcean))
    }

    /// Delete a firewall by ID.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if deletion fails.
    pub fn delete_firewall(&self, firewall_id: &str) -> Result<()> {
        let _ = firewall_id;
        // TODO: Implement `doctl compute firewall delete`.
        Ok(())
    }

    /// Add rules to an existing firewall.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FirewallRuleConflict`] if any rule conflicts.
    pub fn add_rules(&self, firewall_id: &str, rules: &[FirewallRule]) -> Result<()> {
        let _ = (firewall_id, rules);
        // TODO: Implement `doctl compute firewall add-rules`.
        Ok(())
    }

    /// Remove rules from an existing firewall.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if removal fails.
    pub fn remove_rules(&self, firewall_id: &str, rules: &[FirewallRule]) -> Result<()> {
        let _ = (firewall_id, rules);
        // TODO: Implement `doctl compute firewall remove-rules`.
        Ok(())
    }
}

impl Default for DigitalOceanClient {
    fn default() -> Self {
        Self::new()
    }
}
