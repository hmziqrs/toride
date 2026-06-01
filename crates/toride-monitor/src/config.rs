//! Configuration management for toride-monitor.
//!
//! Provides [`MonitorConfig`] for loading, validating, and saving monitoring
//! specifications to disk. Uses TOML-based config files.

use std::path::Path;

use crate::spec::{AlertTarget, AnomalyThreshold, LoggingRule, MonitorSpec};
use crate::validate::{validate_logging_rule, validate_threshold};
use crate::{Error, Result};

/// Configuration file manager for toride-monitor.
///
/// Handles loading and saving [`MonitorSpec`] to a TOML config file,
/// with validation on both load and save.
pub struct MonitorConfig {
    /// Path to the config file.
    path: std::path::PathBuf,
}

impl MonitorConfig {
    /// Create a new config manager pointing at the given file path.
    #[must_use]
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Create a config manager at the default XDG config location.
    ///
    /// The default path is `$XDG_CONFIG_HOME/toride/monitor.toml`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the XDG config directory cannot be resolved.
    pub fn default_config() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| Error::Other("cannot determine config directory".into()))?;
        Ok(Self::new(config_dir.join("toride").join("monitor.toml")))
    }

    /// Load and validate a [`MonitorSpec`] from the config file.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read.
    /// - The file contains invalid TOML.
    /// - Validation of rules or thresholds fails.
    pub fn load(&self) -> Result<MonitorSpec> {
        let content = std::fs::read_to_string(&self.path)?;
        let spec = parse_config(&content)?;
        validate_spec(&spec)?;
        Ok(spec)
    }

    /// Save a [`MonitorSpec`] to the config file.
    ///
    /// Validates the spec before writing.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Validation fails.
    /// - The file cannot be written.
    pub fn save(&self, spec: &MonitorSpec) -> Result<()> {
        validate_spec(spec)?;
        let content = render_config(spec);
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, content)?;
        Ok(())
    }

    /// Returns the config file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check if the config file exists on disk.
    #[must_use]
    pub fn exists(&self) -> bool {
        self.path.exists()
    }
}

/// Parse a TOML config string into a [`MonitorSpec`].
///
/// This is a simplified parser. A full implementation would use a TOML
/// deserialization library.
fn parse_config(_content: &str) -> Result<MonitorSpec> {
    // TODO: Implement TOML parsing with serde + toml crate.
    Ok(MonitorSpec::default())
}

/// Render a [`MonitorSpec`] into a TOML config string.
fn render_config(spec: &MonitorSpec) -> String {
    let mut out = String::new();

    out.push_str("# toride-monitor configuration\n\n");
    out.push_str(&format!("enabled = {}\n\n", spec.enabled));

    out.push_str("[thresholds]\n");
    out.push_str(&format!("max_connections = {}\n", spec.thresholds.max_connections));
    out.push_str(&format!(
        "max_unique_destinations = {}\n",
        spec.thresholds.max_unique_destinations
    ));
    out.push_str(&format!("max_bytes = {}\n", spec.thresholds.max_bytes));
    out.push_str(&format!(
        "max_packets_per_second = {}\n",
        spec.thresholds.max_packets_per_second
    ));
    out.push_str(&format!(
        "window_secs = {}\n",
        spec.thresholds.window.as_secs()
    ));

    out.push_str("\n[[logging_rules]]\n");
    for rule in &spec.logging_rules {
        out.push_str(&format!("name = \"{}\"\n", rule.name));
        out.push_str(&format!("destination = \"{}\"\n", rule.destination));
        out.push_str(&format!("protocol = \"{}\"\n", rule.protocol));
        out.push_str(&format!("log_prefix = \"{}\"\n", rule.log_prefix));
        out.push_str(&format!("log_level = \"{}\"\n", rule.log_level));
        out.push_str(&format!("limit_rate = \"{}\"\n", rule.limit_rate));
        out.push_str(&format!("limit_burst = {}\n", rule.limit_burst));
        out.push_str("\n");
    }

    out
}

/// Validate a complete [`MonitorSpec`].
fn validate_spec(spec: &MonitorSpec) -> Result<()> {
    validate_threshold(&spec.thresholds)?;
    for rule in &spec.logging_rules {
        validate_logging_rule(rule)?;
    }
    Ok(())
}
