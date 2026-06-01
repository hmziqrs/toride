//! AWS EC2 security group management.
//!
//! Provides typed wrappers around the `aws` CLI for managing EC2 security
//! groups, ingress/egress rules, and VPC firewall configuration.

use crate::error::{Error, Result};
use crate::spec::{FirewallRule, SecurityGroup};
use crate::CloudProvider;

// ---------------------------------------------------------------------------
// AwsClient
// ---------------------------------------------------------------------------

/// Client for managing AWS EC2 security groups.
///
/// Delegates command execution to the `aws` CLI. All commands go through the
/// centralised runner pattern for testability.
pub struct AwsClient {
    /// AWS region (e.g. `us-east-1`).
    pub region: String,
    /// AWS profile name (uses default profile if `None`).
    pub profile: Option<String>,
}

impl AwsClient {
    /// Create a new AWS client for the given region.
    #[must_use]
    pub fn new(region: impl Into<String>) -> Self {
        Self {
            region: region.into(),
            profile: None,
        }
    }

    /// Set the AWS profile.
    #[must_use]
    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }

    /// List all security groups in the current region.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the `aws` CLI is not installed
    /// or returns a non-zero exit code.
    pub fn list_security_groups(&self) -> Result<Vec<SecurityGroup>> {
        // TODO: Implement `aws ec2 describe-security-groups` wrapper.
        let _ = &self.region;
        let _ = &self.profile;
        Ok(Vec::new())
    }

    /// Get a security group by ID.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ProviderNotFound`] if the group does not exist.
    pub fn get_security_group(&self, group_id: &str) -> Result<SecurityGroup> {
        let _ = group_id;
        // TODO: Implement `aws ec2 describe-security-groups --group-ids`.
        Err(Error::ProviderNotFound(format!(
            "security group {group_id} not found"
        )))
    }

    /// Create a new security group.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if creation fails.
    pub fn create_security_group(
        &self,
        name: &str,
        description: &str,
        vpc_id: Option<&str>,
    ) -> Result<SecurityGroup> {
        let _ = (name, description, vpc_id);
        // TODO: Implement `aws ec2 create-security-group`.
        Ok(SecurityGroup::new(name, CloudProvider::Aws))
    }

    /// Delete a security group by ID.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if deletion fails.
    pub fn delete_security_group(&self, group_id: &str) -> Result<()> {
        let _ = group_id;
        // TODO: Implement `aws ec2 delete-security-group`.
        Ok(())
    }

    /// Add an ingress rule to a security group.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FirewallRuleConflict`] if the rule conflicts with
    /// an existing rule.
    pub fn authorize_ingress(&self, group_id: &str, rule: &FirewallRule) -> Result<()> {
        let _ = (group_id, rule);
        // TODO: Implement `aws ec2 authorize-security-group-ingress`.
        Ok(())
    }

    /// Remove an ingress rule from a security group.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the rule does not exist.
    pub fn revoke_ingress(&self, group_id: &str, rule: &FirewallRule) -> Result<()> {
        let _ = (group_id, rule);
        // TODO: Implement `aws ec2 revoke-security-group-ingress`.
        Ok(())
    }
}
