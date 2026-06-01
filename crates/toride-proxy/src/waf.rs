//! Web Application Firewall (WAF) stub.
//!
//! This module provides a placeholder for WAF functionality. Future
//! implementations will support rule management, request filtering,
//! and integration with Nginx's `ngx_http_modsecurity_module`.

use crate::error::{Error, Result};

/// WAF rule severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WafSeverity {
    /// Emergency -- critical threats.
    Emergency,
    /// Alert -- high-severity threats.
    Alert,
    /// Critical -- significant threats.
    Critical,
    /// Error -- moderate threats.
    Error,
    /// Warning -- low-severity threats.
    Warning,
    /// Notice -- informational.
    Notice,
    /// Debug -- debugging information.
    Debug,
}

/// A WAF rule definition (stub).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WafRule {
    /// Unique rule identifier (e.g. "900001").
    pub id: String,
    /// Rule description.
    pub description: String,
    /// Rule severity.
    pub severity: WafSeverity,
    /// Whether the rule is currently enabled.
    pub enabled: bool,
}

impl WafRule {
    /// Create a new WAF rule.
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        severity: WafSeverity,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            severity,
            enabled: true,
        }
    }

    /// Disable this rule.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// WAF configuration (stub).
#[derive(Debug, Clone, Default)]
pub struct WafConfig {
    /// Whether WAF is enabled.
    pub enabled: bool,
    /// WAF mode (detection vs. prevention).
    pub mode: WafMode,
    /// Managed rules.
    pub rules: Vec<WafRule>,
}

/// WAF operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WafMode {
    /// Detection only -- log but don't block.
    #[default]
    Detection,
    /// Prevention -- log and block.
    Prevention,
}

impl WafConfig {
    /// Create a new WAF configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable the WAF.
    pub fn enabled(mut self) -> Self {
        self.enabled = true;
        self
    }

    /// Set the WAF mode to prevention.
    pub fn prevention(mut self) -> Self {
        self.mode = WafMode::Prevention;
        self
    }

    /// Add a rule.
    pub fn rule(mut self, rule: WafRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Return enabled rules.
    pub fn enabled_rules(&self) -> Vec<&WafRule> {
        self.rules.iter().filter(|r| r.enabled).collect()
    }

    /// Validate the WAF configuration.
    pub fn validate(&self) -> Result<()> {
        // Check for duplicate rule IDs
        let mut seen = std::collections::HashSet::new();
        for rule in &self.rules {
            if !seen.insert(&rule.id) {
                return Err(Error::Validation(format!(
                    "duplicate WAF rule ID: {}",
                    rule.id
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waf_config_builder() {
        let config = WafConfig::new()
            .enabled()
            .prevention()
            .rule(WafRule::new("900001", "Block SQL injection", WafSeverity::Critical))
            .rule(WafRule::new("900002", "Block XSS", WafSeverity::Critical).disabled());

        assert!(config.enabled);
        assert_eq!(config.mode, WafMode::Prevention);
        assert_eq!(config.rules.len(), 2);
        assert_eq!(config.enabled_rules().len(), 1);
    }

    #[test]
    fn waf_config_rejects_duplicate_ids() {
        let config = WafConfig::new()
            .rule(WafRule::new("900001", "Rule A", WafSeverity::Critical))
            .rule(WafRule::new("900001", "Rule B", WafSeverity::Alert));

        assert!(config.validate().is_err());
    }
}
