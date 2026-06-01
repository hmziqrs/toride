//! Monitoring specification types.
//!
//! [`MonitorSpec`] defines the complete configuration for outbound traffic
//! monitoring: logging rules, anomaly thresholds, and alert targets.

use std::time::Duration;

// ---------------------------------------------------------------------------
// LoggingRule
// ---------------------------------------------------------------------------

/// A single iptables OUTPUT chain logging rule.
///
/// Describes which outbound traffic to log via the iptables `LOG` target.
#[derive(Debug, Clone)]
pub struct LoggingRule {
    /// Human-readable name for this rule.
    pub name: String,
    /// Destination CIDR or host to match (e.g. `"0.0.0.0/0"` for all).
    pub destination: String,
    /// Destination port to match, or `None` for any port.
    pub dest_port: Option<u16>,
    /// Protocol to match (e.g. `"tcp"`, `"udp"`).
    pub protocol: String,
    /// Log prefix written to kernel log (max 29 characters).
    pub log_prefix: String,
    /// Log level (e.g. `"info"`, `"warning"`).
    pub log_level: String,
    /// Maximum burst rate before logging is rate-limited.
    pub limit_burst: u32,
    /// Rate limit for log entries (e.g. `"10/minute"`).
    pub limit_rate: String,
}

// ---------------------------------------------------------------------------
// AnomalyThreshold
// ---------------------------------------------------------------------------

/// Threshold configuration for anomaly detection.
///
/// When monitored metrics exceed these values, an anomaly is flagged.
#[derive(Debug, Clone)]
pub struct AnomalyThreshold {
    /// Maximum number of concurrent outbound connections before flagging.
    pub max_connections: u64,
    /// Maximum number of unique destination IPs in the sampling window.
    pub max_unique_destinations: u64,
    /// Maximum bytes transferred in the sampling window.
    pub max_bytes: u64,
    /// Maximum packets per second before flagging.
    pub max_packets_per_second: u64,
    /// Duration of the sliding sampling window.
    pub window: Duration,
}

impl Default for AnomalyThreshold {
    fn default() -> Self {
        Self {
            max_connections: 500,
            max_unique_destinations: 200,
            max_bytes: 100 * 1024 * 1024, // 100 MB
            max_packets_per_second: 10_000,
            window: Duration::from_secs(60),
        }
    }
}

// ---------------------------------------------------------------------------
// AlertTarget
// ---------------------------------------------------------------------------

/// An alert dispatch target.
///
/// Defines where anomaly alerts are sent when triggered.
#[derive(Debug, Clone)]
pub enum AlertTarget {
    /// Send alerts to the systemd journal.
    Journald {
        /// Priority level (e.g. `"warning"`, `"crit"`).
        priority: String,
    },
    /// Send alerts to a webhook endpoint.
    Webhook {
        /// URL to POST alert payloads to.
        url: String,
        /// Custom HTTP headers to include.
        headers: Vec<(String, String)>,
    },
    /// Write alerts to a log file.
    File {
        /// File path to append alert entries.
        path: String,
    },
}

// ---------------------------------------------------------------------------
// MonitorSpec
// ---------------------------------------------------------------------------

/// Complete monitoring specification.
///
/// Aggregates logging rules, anomaly thresholds, and alert targets into a
/// single configuration object.
#[derive(Debug, Clone)]
pub struct MonitorSpec {
    /// Logging rules to apply to the iptables OUTPUT chain.
    pub logging_rules: Vec<LoggingRule>,
    /// Thresholds for anomaly detection.
    pub thresholds: AnomalyThreshold,
    /// Alert dispatch targets.
    pub alert_targets: Vec<AlertTarget>,
    /// Whether monitoring is enabled.
    pub enabled: bool,
}

impl Default for MonitorSpec {
    fn default() -> Self {
        Self {
            logging_rules: Vec::new(),
            thresholds: AnomalyThreshold::default(),
            alert_targets: vec![AlertTarget::Journald {
                priority: "warning".to_owned(),
            }],
            enabled: true,
        }
    }
}
