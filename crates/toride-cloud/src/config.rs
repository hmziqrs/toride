//! Configuration management for cloud provider settings.
//!
//! Handles loading, parsing, validating, and saving cloud provider
//! configuration files. Configuration is stored as TOML or JSON in the
//! XDG config directory.

use std::path::PathBuf;

use crate::error::{Error, Result};
use crate::paths::CloudPaths;
use crate::CloudProvider;

// ---------------------------------------------------------------------------
// CloudConfig
// ---------------------------------------------------------------------------

/// Cloud provider configuration.
///
/// Represents the persisted configuration for cloud provider integration,
/// including provider selection, default regions, and security group defaults.
#[derive(Debug, Clone)]
pub struct CloudConfig {
    /// The configured cloud provider.
    pub provider: CloudProvider,
    /// Default region for the provider.
    pub default_region: String,
    /// Path to the configuration file.
    pub config_path: PathBuf,
    /// Whether to automatically detect the provider on startup.
    pub auto_detect: bool,
    /// Default action for new firewall rules.
    pub default_action: crate::spec::RuleAction,
}

impl CloudConfig {
    /// Load configuration from the default XDG location.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ConfigParse`] if the file exists but cannot be parsed.
    /// Returns [`Error::Io`] if the file cannot be read.
    pub fn load() -> Result<Self> {
        let paths = CloudPaths::default()?;
        let config_path = paths.config_dir.join("config.json");

        if !config_path.exists() {
            return Self::default_config(config_path);
        }

        let content = std::fs::read_to_string(&config_path)?;
        Self::parse(&content, config_path)
    }

    /// Load configuration from a specific file path.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ConfigParse`] if the file cannot be parsed.
    pub fn load_from(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            return Self::default_config(path);
        }

        let content = std::fs::read_to_string(&path)?;
        Self::parse(&content, path)
    }

    /// Parse configuration from a string.
    fn parse(content: &str, config_path: PathBuf) -> Result<Self> {
        let _ = content;
        // TODO: Implement JSON/TOML parsing with regex validation.
        Ok(Self {
            provider: CloudProvider::Unknown,
            default_region: String::new(),
            config_path,
            auto_detect: true,
            default_action: crate::spec::RuleAction::Allow,
        })
    }

    /// Create a default configuration.
    fn default_config(config_path: PathBuf) -> Result<Self> {
        Ok(Self {
            provider: CloudProvider::Unknown,
            default_region: String::new(),
            config_path,
            auto_detect: true,
            default_action: crate::spec::RuleAction::Allow,
        })
    }

    /// Save the configuration to disk.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the file cannot be written.
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // TODO: Implement JSON/TOML serialization.
        Ok(())
    }

    /// Validate the configuration.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ConfigParse`] if the configuration is invalid.
    pub fn validate(&self) -> Result<()> {
        if matches!(self.provider, CloudProvider::Unknown) && !self.auto_detect {
            return Err(Error::ConfigParse(
                "provider is unknown and auto_detect is disabled".to_string(),
            ));
        }
        Ok(())
    }
}
