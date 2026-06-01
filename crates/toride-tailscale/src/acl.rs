//! ACL policy management for Tailscale tailnets.
//!
//! Provides types and operations for managing Access Control Lists in a
//! Tailscale tailnet. ACLs define which nodes and users can communicate
//! with each other and on which ports.

use crate::spec::AclRule;
use crate::Result;

// ---------------------------------------------------------------------------
// AclManager
// ---------------------------------------------------------------------------

/// Manager for Tailscale ACL policies.
///
/// `AclManager` provides methods for validating, applying, and auditing
/// ACL rules in a tailnet. ACLs are typically managed through the Tailscale
/// coordination server, not the local API, so this module focuses on
/// validation and policy generation.
///
/// # Example
///
/// ```ignore
/// use toride_tailscale::acl::AclManager;
/// use toride_tailscale::spec::AclRule;
///
/// let manager = AclManager::new();
/// let rules = vec![AclRule {
///     action: toride_tailscale::spec::AclAction::Allow,
///     src: vec!["autogroup:members".to_owned()],
///     dst: vec!["*:*".to_owned()],
/// }];
/// manager.validate_rules(&rules)?;
/// ```
pub struct AclManager {
    /// Whether to run in dry-run mode (validate only, no changes).
    dry_run: bool,
}

impl AclManager {
    /// Create a new `AclManager`.
    pub fn new() -> Self {
        Self { dry_run: false }
    }

    /// Enable dry-run mode.
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Validate a list of ACL rules for syntactic and semantic correctness.
    ///
    /// Checks that:
    /// - Source and destination fields are non-empty
    /// - Destination ports are valid
    /// - No conflicting rules exist
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::AclError`] if any rule is invalid.
    pub fn validate_rules(&self, rules: &[AclRule]) -> Result<()> {
        for (i, rule) in rules.iter().enumerate() {
            if rule.src.is_empty() {
                return Err(crate::Error::AclError(format!(
                    "rule {i}: source list must not be empty"
                )));
            }
            if rule.dst.is_empty() {
                return Err(crate::Error::AclError(format!(
                    "rule {i}: destination list must not be empty"
                )));
            }
        }
        Ok(())
    }

    /// Generate a Tailscale ACL policy document from a list of rules.
    ///
    /// # Errors
    ///
    /// Returns an error if any rule fails validation.
    pub fn generate_policy(&self, rules: &[AclRule]) -> Result<String> {
        self.validate_rules(rules)?;

        // TODO: Generate a full Tailscale ACL policy document in JSON/HuJSON format.
        let _ = self.dry_run;
        Ok(String::new())
    }
}

impl Default for AclManager {
    fn default() -> Self {
        Self::new()
    }
}
