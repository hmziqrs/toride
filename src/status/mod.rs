//! Toride status subsystem.
//!
//! Provides [`TorideStatus`] — a point-in-time snapshot of every monitored
//! subsystem (OS metrics, daemon liveness, SSH health).
//!
//! ```no_run
//! use toride::status::TorideStatus;
//!
//! let status = TorideStatus::collect();
//! println!("{status}");
//! ```

pub mod capabilities;
pub mod collector;
pub mod daemon;
pub mod doctor;
pub mod error;
pub mod presets;
pub mod privacy;
pub mod provider;
pub mod ssh;
pub mod system;

use std::fmt;

use serde::Serialize;

pub use capabilities::Capabilities;
pub use collector::{Collector, SystemDelta};
pub use daemon::DaemonStatus;
pub use doctor::{CheckStatus, DoctorCheck, DoctorReport};
pub use error::{StatusError, StatusResult};
pub use presets::Preset;
pub use privacy::{PrivacyMode, Redactor};
pub use ssh::SshStatus;
pub use system::SystemStatus;

/// Top-level aggregated status snapshot.
///
/// Collects data from all subsystems in a single [`collect`](Self::collect)
/// call. Each sub-status is independent — a failure in one subsystem does not
/// prevent the others from being collected.
///
/// # Examples
///
/// ```no_run
/// use toride::status::TorideStatus;
///
/// let status = TorideStatus::collect();
/// assert!(!status.system.hostname.is_empty());
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct TorideStatus {
    /// OS-level metrics (CPU, memory, disk, network, uptime).
    pub system: SystemStatus,
    /// Daemon liveness and health (PID, restart count, socket state).
    pub daemon: DaemonStatus,
    /// SSH subsystem health (mux master, control path, agent, keys).
    pub ssh: SshStatus,
    /// Platform capabilities.
    pub capabilities: Capabilities,
    /// Non-fatal warnings collected during status gathering.
    #[serde(skip)]
    pub warnings: Vec<StatusError>,
}

impl TorideStatus {
    /// Collect a point-in-time snapshot of all subsystems.
    ///
    /// Each subsystem is collected independently — if one fails, its fields
    /// will contain `None` values rather than propagating the error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use toride::status::TorideStatus;
    ///
    /// let status = TorideStatus::collect();
    /// println!("{}", status.system.hostname);
    /// ```
    #[must_use]
    pub fn collect() -> Self {
        let system = SystemStatus::collect();
        let daemon = DaemonStatus::collect();
        let ssh = SshStatus::collect();
        let capabilities = Capabilities::detect();
        let mut warnings = Vec::new();
        if system.hostname.is_empty() {
            warnings.push(StatusError::DataUnavailable("hostname unavailable".to_string()));
        }
        if system.memory.total_bytes == 0 {
            warnings.push(StatusError::DataUnavailable("memory info unavailable".to_string()));
        }
        Self { system, daemon, ssh, capabilities, warnings }
    }
}

impl fmt::Display for TorideStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== Toride Status ===")?;
        write!(f, "{}", self.system)?;
        write!(f, "{}", self.daemon)?;
        write!(f, "{}", self.ssh)?;
        write!(f, "{}", self.capabilities)?;
        if !self.warnings.is_empty() {
            writeln!(f, "Warnings:")?;
            for w in &self.warnings {
                writeln!(f, "  \u{26a0} {w}")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_returns_all_subsystems() {
        let status = TorideStatus::collect();
        // SystemStatus should always have a hostname on any platform
        assert!(
            !status.system.hostname.is_empty(),
            "hostname should not be empty"
        );
    }

    #[test]
    fn display_contains_section_headers() {
        let status = TorideStatus::collect();
        let output = format!("{status}");
        assert!(output.contains("=== Toride Status ==="));
        assert!(output.contains("System:"));
        assert!(output.contains("Daemon:"));
        assert!(output.contains("SSH:"));
        if !status.warnings.is_empty() {
            assert!(output.contains("Warnings:"));
        }
    }

    #[test]
    fn serialize_to_json_succeeds() {
        let status = TorideStatus::collect();
        let json = serde_json::to_string(&status);
        assert!(json.is_ok(), "serialization should succeed: {:?}", json.err());
    }

    #[test]
    fn snapshot_toride_status_display() {
        use crate::status::system::{
            CpuCore, DiskStatus, LoadAverage, MemoryStatus, NetworkInterface, NetworkStatus,
            OsInfo, SensorStatus, SwapStatus,
        };
        let status = TorideStatus {
            system: SystemStatus {
                cpu_usage: Some(42.5),
                memory: MemoryStatus {
                    used_bytes: 8 * 1024 * 1024 * 1024,
                    total_bytes: 16 * 1024 * 1024 * 1024,
                    percentage: 50.0,
                },
                disk: DiskStatus {
                    name: "Macintosh HD".to_string(),
                    mount_point: "/".to_string(),
                    filesystem: "apfs".to_string(),
                    used_bytes: 500 * 1024 * 1024 * 1024,
                    total_bytes: 1024 * 1024 * 1024 * 1024,
                    percentage: 50.0,
                    is_removable: false,
                },
                network: NetworkStatus {
                    bytes_received: 100 * 1024 * 1024 * 1024,
                    bytes_transmitted: 50 * 1024 * 1024 * 1024,
                },
                load_average: Some(LoadAverage {
                    one: 2.50,
                    five: 3.00,
                    fifteen: 3.50,
                }),
                uptime_secs: Some(90061),
                hostname: "test-host".to_string(),
                os_info: OsInfo {
                    name: Some("macOS".to_string()),
                    version: Some("15.0".to_string()),
                    kernel_version: Some("24.0.0".to_string()),
                    arch: "aarch64".to_string(),
                },
                cpu_cores: vec![
                    CpuCore {
                        name: "cpu0".to_string(),
                        usage: 45.0,
                        frequency: 3200,
                    },
                    CpuCore {
                        name: "cpu1".to_string(),
                        usage: 40.0,
                        frequency: 3200,
                    },
                ],
                physical_cores: Some(8),
                swap: Some(SwapStatus {
                    used_bytes: 512 * 1024 * 1024,
                    total_bytes: 2 * 1024 * 1024 * 1024,
                    percentage: 25.0,
                }),
                disks: vec![],
                network_interfaces: vec![
                    NetworkInterface {
                        name: "en0".to_string(),
                        bytes_received: 60 * 1024 * 1024 * 1024,
                        bytes_transmitted: 30 * 1024 * 1024 * 1024,
                        packets_received: 1000000,
                        packets_transmitted: 500000,
                        errors_received: 0,
                        errors_transmitted: 0,
                    },
                ],
                sensors: vec![
                    SensorStatus {
                        label: "CPU".to_string(),
                        temperature: Some(55.5),
                    },
                ],
                boot_time: Some(1700000000),
                processes: crate::status::system::ProcessSnapshot {
                    processes: vec![],
                    total_count: 0,
                },
                gpu: vec![],
                battery: None,
            },
            daemon: DaemonStatus {
                alive: true,
                pid: Some(54321),
                uptime_secs: Some(86400),
                restart_count: 2,
                stale_socket: false,
            },
            ssh: SshStatus {
                mux_master_alive: true,
                control_path_valid: true,
                config_valid: true,
                agent_running: true,
                key_count: 3,
            },
            capabilities: Capabilities::detect(),
            warnings: vec![],
        };
        insta::assert_snapshot!("toride_status_display", format!("{}", status));
    }

    #[test]
    fn collect_includes_capabilities() {
        let status = TorideStatus::collect();
        assert!(status.capabilities.system.cpu_usage);
    }

    #[test]
    fn collect_includes_processes() {
        let status = TorideStatus::collect();
        assert!(status.system.processes.total_count > 0);
    }

    #[test]
    fn collector_produces_status_and_delta() {
        let mut collector = Collector::default_collector();
        let (status, delta) = collector.collect();
        assert!(!status.system.hostname.is_empty());
        assert!(delta.is_none()); // first collect
        std::thread::sleep(std::time::Duration::from_millis(50));
        let (_, delta2) = collector.collect();
        assert!(delta2.is_some());
    }

    #[test]
    fn redactor_safe_mode() {
        let r = Redactor::new(PrivacyMode::Safe);
        assert_eq!(r.redact_hostname("myhost"), "[redacted]");
    }

    #[test]
    fn preset_diagnostics_includes_all() {
        let p = Preset::Diagnostics;
        assert!(p.includes_per_core_cpu());
        assert!(p.includes_processes());
    }

    // ── Integration: Full pipeline ─────────────────────────────────

    #[test]
    fn integration_full_pipeline_collect_serialize_display() {
        // Collect a full TorideStatus snapshot.
        let status = TorideStatus::collect();

        // Verify all subsystems are populated with non-trivial data.
        assert!(!status.system.hostname.is_empty(), "system hostname must be populated");
        assert!(status.system.memory.total_bytes > 0, "memory total must be nonzero");
        assert!(status.system.disk.total_bytes > 0 || !status.system.disk.mount_point.is_empty(),
            "disk must be populated");
        assert!(!status.system.os_info.arch.is_empty(), "OS arch must be populated");
        assert!(status.system.processes.total_count > 0, "process count must be nonzero");
        // daemon and ssh fields are always set (even if not alive/running).
        // capabilities always populated via detect().
        assert!(status.capabilities.system.cpu_usage, "capabilities must report cpu_usage");

        // Serialize to JSON and verify it parses as valid JSON.
        let json = serde_json::to_string(&status).expect("serialization must succeed");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("JSON must be valid and parseable");
        assert!(parsed.is_object(), "JSON must be an object");
        assert!(parsed.get("system").is_some(), "JSON must contain 'system' key");
        assert!(parsed.get("daemon").is_some(), "JSON must contain 'daemon' key");
        assert!(parsed.get("ssh").is_some(), "JSON must contain 'ssh' key");
        assert!(parsed.get("capabilities").is_some(), "JSON must contain 'capabilities' key");

        // Display and verify all section headers are present.
        let display = format!("{status}");
        assert!(display.contains("=== Toride Status ==="), "display must have top header");
        assert!(display.contains("System:"), "display must have System section");
        assert!(display.contains("Daemon:"), "display must have Daemon section");
        assert!(display.contains("SSH:"), "display must have SSH section");
        assert!(display.contains("Capabilities"), "display must have Capabilities section");
    }

    // ── Integration: Collector ─────────────────────────────────────

    #[test]
    fn integration_collector_two_collects_with_delta() {
        let mut collector = Collector::default_collector();

        // First collect: status present, delta absent.
        let (status1, delta1) = collector.collect();
        assert!(!status1.system.hostname.is_empty(), "first collect must return valid status");
        assert!(delta1.is_none(), "first collect must have no delta");

        // Sleep briefly so elapsed > 0 for the delta.
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Second collect: delta present with reasonable values.
        let (status2, delta2) = collector.collect();
        assert!(!status2.system.hostname.is_empty(), "second collect must return valid status");
        let d = delta2.expect("second collect must produce a delta");

        // Elapsed must be at least as long as we slept.
        assert!(d.elapsed >= std::time::Duration::from_millis(80),
            "delta elapsed ({:?}) must be >= 80ms", d.elapsed);

        // Rates must be non-negative and finite.
        assert!(d.bytes_received_rate.is_finite(), "RX rate must be finite");
        assert!(d.bytes_received_rate >= 0.0, "RX rate must be non-negative");
        assert!(d.bytes_transmitted_rate.is_finite(), "TX rate must be finite");
        assert!(d.bytes_transmitted_rate >= 0.0, "TX rate must be non-negative");

        // Deltas must be non-negative (saturating_sub).
        // (They could be 0 if the system had no traffic, which is fine.)

        // CPU delta: if both snapshots had CPU data, the delta should be Some.
        if status1.system.cpu_usage.is_some() && status2.system.cpu_usage.is_some() {
            assert!(d.cpu_usage_delta.is_some(), "CPU delta must be Some when both snapshots have CPU data");
        }
    }

    // ── Integration: Privacy ───────────────────────────────────────

    #[test]
    fn integration_privacy_redactor_on_toride_hostname() {
        let status = TorideStatus::collect();
        let hostname = &status.system.hostname;
        assert!(!hostname.is_empty(), "hostname must be non-empty for this test");

        // Safe mode: hostname is fully redacted.
        let safe = Redactor::new(PrivacyMode::Safe);
        let redacted = safe.redact_hostname(hostname);
        assert_eq!(redacted, "[redacted]", "Safe mode must redact hostname");
        assert_ne!(redacted, *hostname, "redacted value must differ from original");

        // Diagnostics mode: hostname is shown as-is.
        let diag = Redactor::new(PrivacyMode::Diagnostics);
        let shown = diag.redact_hostname(hostname);
        assert_eq!(shown, *hostname, "Diagnostics mode must preserve hostname");

        // Full mode: hostname is also shown.
        let full = Redactor::new(PrivacyMode::Full);
        let full_shown = full.redact_hostname(hostname);
        assert_eq!(full_shown, *hostname, "Full mode must preserve hostname");
    }

    // ── Integration: Presets ───────────────────────────────────────

    #[test]
    fn integration_preset_diagnostics_includes_all_features() {
        let p = Preset::Diagnostics;
        assert!(p.includes_per_core_cpu(), "Diagnostics must include per-core CPU");
        assert!(p.includes_swap(), "Diagnostics must include swap");
        assert!(p.includes_sensors(), "Diagnostics must include sensors");
        assert!(p.includes_processes(), "Diagnostics must include processes");
        assert!(p.includes_network_interfaces(), "Diagnostics must include network interfaces");
        assert!(p.includes_all_disks(), "Diagnostics must include all disks");
        assert!(p.includes_os_info(), "Diagnostics must include OS info");
    }

    #[test]
    fn integration_preset_minimal_excludes_features() {
        let p = Preset::Minimal;
        assert!(!p.includes_per_core_cpu(), "Minimal must exclude per-core CPU");
        assert!(!p.includes_swap(), "Minimal must exclude swap");
        assert!(!p.includes_sensors(), "Minimal must exclude sensors");
        assert!(!p.includes_processes(), "Minimal must exclude processes");
        assert!(!p.includes_network_interfaces(), "Minimal must exclude network interfaces");
        assert!(!p.includes_all_disks(), "Minimal must exclude all disks");
        // Minimal always includes OS info.
        assert!(p.includes_os_info(), "Minimal must include OS info");
    }

    // ── Integration: Doctor ────────────────────────────────────────

    #[test]
    fn integration_doctor_report_has_system_daemon_ssh_checks() {
        let report = DoctorReport::check();
        assert!(!report.checks.is_empty(), "doctor report must have checks");

        // Verify checks exist for each subsystem category.
        let has_system = report.checks.iter().any(|c| c.name.starts_with("system."));
        let has_daemon = report.checks.iter().any(|c| c.name.starts_with("daemon."));
        let has_ssh = report.checks.iter().any(|c| c.name.starts_with("ssh."));
        assert!(has_system, "report must include system checks");
        assert!(has_daemon, "report must include daemon checks");
        assert!(has_ssh, "report must include ssh checks");

        // Verify summary counts are consistent.
        let (pass, warn, fail) = report.summary();
        assert_eq!(
            pass + warn + fail,
            report.checks.len(),
            "summary counts must equal total check count"
        );
    }

    // ── Integration: Error handling ────────────────────────────────

    #[test]
    fn integration_error_variants_work() {
        // PermissionDenied
        let err = StatusError::PermissionDenied("/secret".into());
        assert!(err.to_string().contains("permission denied"));
        assert!(err.to_string().contains("/secret"));

        // CommandNotFound
        let err = StatusError::CommandNotFound("foobar".into());
        assert!(err.to_string().contains("command not found"));
        assert!(err.to_string().contains("foobar"));

        // CommandFailed
        let err = StatusError::CommandFailed {
            command: "ls".into(),
            code: 1,
            stderr: "no such file".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("command failed"), "msg: {msg}");
        assert!(msg.contains("exited 1"), "msg: {msg}");
        assert!(msg.contains("no such file"), "msg: {msg}");

        // CommandTimeout
        let err = StatusError::CommandTimeout("ping".into());
        assert!(err.to_string().contains("timed out"));

        // ParseError
        let err = StatusError::ParseError("bad data".into());
        assert!(err.to_string().contains("parse error"));

        // Io
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err = StatusError::Io(io_err);
        assert!(err.to_string().contains("io error"));

        // Unsupported
        let err = StatusError::Unsupported("plan9".into());
        assert!(err.to_string().contains("unsupported platform"));

        // DataUnavailable
        let err = StatusError::DataUnavailable("gpu".into());
        assert!(err.to_string().contains("data unavailable"));
    }

    #[test]
    fn integration_error_clone_works_for_all_variants() {
        let variants = vec![
            StatusError::PermissionDenied("path".into()),
            StatusError::CommandNotFound("cmd".into()),
            StatusError::CommandFailed {
                command: "run".into(),
                code: 42,
                stderr: "err".into(),
            },
            StatusError::CommandTimeout("slow".into()),
            StatusError::ParseError("bad".into()),
            StatusError::Io(std::io::Error::new(std::io::ErrorKind::Other, "io")),
            StatusError::Unsupported("os".into()),
            StatusError::DataUnavailable("info".into()),
        ];

        for original in &variants {
            let cloned = original.clone();
            // Cloned error must have the same Display output.
            assert_eq!(
                original.to_string(),
                cloned.to_string(),
                "Clone must preserve Display output for variant"
            );
        }
    }

    #[test]
    fn integration_error_display_produces_nonempty_strings() {
        let variants = vec![
            StatusError::PermissionDenied("p".into()),
            StatusError::CommandNotFound("c".into()),
            StatusError::CommandFailed { command: "r".into(), code: 1, stderr: "e".into() },
            StatusError::CommandTimeout("t".into()),
            StatusError::ParseError("p".into()),
            StatusError::Io(std::io::Error::new(std::io::ErrorKind::Other, "o")),
            StatusError::Unsupported("u".into()),
            StatusError::DataUnavailable("d".into()),
        ];

        for variant in &variants {
            let display = variant.to_string();
            assert!(!display.is_empty(), "Display must produce non-empty string for {variant:?}");
            // Every Display string must be at least as long as the prefix.
            assert!(display.len() >= 5, "Display string suspiciously short: {display}");
        }
    }
}
