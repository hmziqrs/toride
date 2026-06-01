//! Hetzner Cloud firewall management.
//!
//! Provides typed wrappers around the `hcloud` CLI for managing Hetzner Cloud
//! firewalls and their rules.

use crate::error::{Error, Result};
use crate::spec::{FirewallRule, SecurityGroup};
use crate::CloudProvider;

// ---------------------------------------------------------------------------
// HetznerClient
// ---------------------------------------------------------------------------

/// Client for managing Hetzner Cloud firewalls.
///
/// Delegates command execution to the `hcloud` CLI.
pub struct HetznerClient {
    /// Hetzner Cloud API token (uses `hcloud` config if `None`).
    pub api_token: Option<String>,
}

impl HetznerClient {
    /// Create a new Hetzner client.
    #[must_use]
    pub fn new() -> Self {
        Self { api_token: None }
    }

    /// Set the API token explicitly.
    #[must_use]
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.api_token = Some(token.into());
        self
    }

    /// List all firewalls in the project.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the `hcloud` CLI is not installed
    /// or returns a non-zero exit code.
    pub fn list_firewalls(&self) -> Result<Vec<SecurityGroup>> {
        // TODO: Implement `hcloud firewall list -o json`.
        let _ = &self.api_token;
        Ok(Vec::new())
    }

    /// Get a firewall by name or ID.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ProviderNotFound`] if the firewall does not exist.
    pub fn get_firewall(&self, name_or_id: &str) -> Result<SecurityGroup> {
        let _ = name_or_id;
        // TODO: Implement `hcloud firewall describe`.
        Err(Error::ProviderNotFound(format!(
            "firewall {name_or_id} not found"
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
        rules: &[FirewallRule],
    ) -> Result<SecurityGroup> {
        let _ = (name, rules);
        // TODO: Implement `hcloud firewall create`.
        Ok(SecurityGroup::new(name, CloudProvider::Hetzner))
    }

    /// Delete a firewall by name or ID.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if deletion fails.
    pub fn delete_firewall(&self, name_or_id: &str) -> Result<()> {
        let _ = name_or_id;
        // TODO: Implement `hcloud firewall delete`.
        Ok(())
    }

    /// Add rules to an existing firewall.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FirewallRuleConflict`] if any rule conflicts.
    pub fn add_rules(&self, firewall_name: &str, rules: &[FirewallRule]) -> Result<()> {
        let _ = (firewall_name, rules);
        // TODO: Implement `hcloud firewall add-rules`.
        Ok(())
    }

    /// Remove rules from an existing firewall.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if removal fails.
    pub fn remove_rules(&self, firewall_name: &str, rules: &[FirewallRule]) -> Result<()> {
        let _ = (firewall_name, rules);
        // TODO: Implement `hcloud firewall remove-rules`.
        Ok(())
    }

    /// Apply a firewall to a server.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the apply fails.
    pub fn apply_to_server(&self, firewall_name: &str, server_name: &str) -> Result<()> {
        let _ = (firewall_name, server_name);
        // TODO: Implement `hcloud firewall apply-to-server`.
        Ok(())
    }

    /// Remove a firewall from a server.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the removal fails.
    pub fn remove_from_server(&self, firewall_name: &str, server_name: &str) -> Result<()> {
        let _ = (firewall_name, server_name);
        // TODO: Implement `hcloud firewall remove-from-server`.
        Ok(())
    }
}

impl Default for HetznerClient {
    fn default() -> Self {
        Self::new()
    }
}
