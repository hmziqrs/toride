//! auditd.conf parsing and management.
//!
//! Provides types and functions for reading, parsing, and writing the
//! audit daemon configuration file (`/etc/audit/auditd.conf`).

use std::collections::BTreeMap;

use crate::Result;

// ---------------------------------------------------------------------------
// AuditdConfig
// ---------------------------------------------------------------------------

/// Parsed representation of `auditd.conf`.
///
/// The configuration file uses `key = value` pairs. This struct provides
/// typed access to the most commonly used settings while preserving
/// all keys in the [`Self::extra`] map for forward compatibility.
#[derive(Debug, Clone)]
pub struct AuditdConfig {
    /// Maximum log file size in megabytes.
    pub max_log_file: Option<u64>,
    /// Action when the log file reaches max size: `ignore`, `syslog`, `suspend`, `rotate`, `keep_logs`.
    pub max_log_file_action: Option<String>,
    /// Number of log files to retain when rotating.
    pub num_logs: Option<u32>,
    /// Log file format: `raw`, `nolog`.
    pub log_format: Option<String>,
    /// Flush mode: `none`, `incremental`, `data`, `sync`.
    pub flush: Option<String>,
    /// Priority boost for the audit daemon.
    pub priority_boost: Option<u32>,
    /// Any additional key-value pairs not covered by typed fields.
    pub extra: BTreeMap<String, String>,
}

impl AuditdConfig {
    /// Parse an `auditd.conf` file content.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ConfigParse`] if the content cannot be parsed.
    pub fn parse(content: &str) -> Result<Self> {
        let mut config = Self {
            max_log_file: None,
            max_log_file_action: None,
            num_logs: None,
            log_format: None,
            flush: None,
            priority_boost: None,
            extra: BTreeMap::new(),
        };

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = trimmed.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "max_log_file" => {
                        config.max_log_file = value.parse().ok();
                    }
                    "max_log_file_action" => {
                        config.max_log_file_action = Some(value.to_owned());
                    }
                    "num_logs" => {
                        config.num_logs = value.parse().ok();
                    }
                    "log_format" => {
                        config.log_format = Some(value.to_owned());
                    }
                    "flush" => {
                        config.flush = Some(value.to_owned());
                    }
                    "priority_boost" => {
                        config.priority_boost = value.parse().ok();
                    }
                    _ => {
                        config.extra.insert(key.to_owned(), value.to_owned());
                    }
                }
            }
        }

        Ok(config)
    }

    /// Render the configuration back to a string suitable for writing to
    /// `auditd.conf`.
    #[must_use]
    pub fn render(&self) -> String {
        let mut lines = Vec::new();

        if let Some(v) = &self.max_log_file {
            lines.push(format!("max_log_file = {v}"));
        }
        if let Some(v) = &self.max_log_file_action {
            lines.push(format!("max_log_file_action = {v}"));
        }
        if let Some(v) = &self.num_logs {
            lines.push(format!("num_logs = {v}"));
        }
        if let Some(v) = &self.log_format {
            lines.push(format!("log_format = {v}"));
        }
        if let Some(v) = &self.flush {
            lines.push(format!("flush = {v}"));
        }
        if let Some(v) = &self.priority_boost {
            lines.push(format!("priority_boost = {v}"));
        }

        for (key, value) in &self.extra {
            lines.push(format!("{key} = {value}"));
        }

        lines.join("\n")
    }

    /// Returns an empty configuration with sensible defaults.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self {
            max_log_file: Some(100),
            max_log_file_action: Some("rotate".to_owned()),
            num_logs: Some(10),
            log_format: Some("raw".to_owned()),
            flush: Some("incremental_async".to_owned()),
            priority_boost: Some(4),
            extra: BTreeMap::new(),
        }
    }
}
