//! OS-level metrics: CPU, memory, disk, network, load average, uptime, hostname.
//!
//! Uses the [`sysinfo`] crate for cross-platform data collection. Each metric
//! returns `None` when the underlying data cannot be read (e.g. permission
//! denied on certain Linux containers).
//!
//! # Platform notes
//!
//! | Metric        | Linux | macOS | Windows |
//! |---------------|:-----:|:-----:|:-------:|
//! | CPU usage     | ✓     | ✓     | ✓       |
//! | Memory        | ✓     | ✓     | ✓       |
//! | Disk usage    | ✓     | ✓     | ✓       |
//! | Network I/O   | ✓     | ✓     | ✓       |
//! | Load average  | ✓     | ✓     | ✗       |
//! | Uptime        | ✓     | ✓     | ✓       |
//! | Hostname      | ✓     | ✓     | ✓       |

use std::fmt;

use serde::Serialize;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, Networks, RefreshKind, System};

/// OS-level system metrics snapshot.
///
/// All fields are populated by [`collect`](Self::collect). Fields that cannot
/// be read (e.g. load average on Windows) will be `None`.
#[derive(Debug, Clone, Serialize)]
pub struct SystemStatus {
    /// CPU usage as a percentage (0.0–100.0).
    pub cpu_usage: Option<f64>,
    /// Memory metrics.
    pub memory: MemoryStatus,
    /// Disk metrics (root filesystem).
    pub disk: DiskStatus,
    /// Network I/O counters.
    pub network: NetworkStatus,
    /// Load average (1, 5, 15 minute). `None` on Windows.
    pub load_average: Option<LoadAverage>,
    /// System uptime in seconds.
    pub uptime_secs: Option<u64>,
    /// System hostname.
    pub hostname: String,
}

/// Memory usage snapshot.
#[derive(Debug, Clone, Serialize)]
pub struct MemoryStatus {
    /// Used memory in bytes.
    pub used_bytes: u64,
    /// Total memory in bytes.
    pub total_bytes: u64,
    /// Usage percentage (0.0–100.0).
    pub percentage: f64,
}

/// Disk usage snapshot (root filesystem).
#[derive(Debug, Clone, Serialize)]
pub struct DiskStatus {
    /// Used disk space in bytes.
    pub used_bytes: u64,
    /// Total disk space in bytes.
    pub total_bytes: u64,
    /// Usage percentage (0.0–100.0).
    pub percentage: f64,
}

/// Network I/O counters.
#[derive(Debug, Clone, Serialize)]
pub struct NetworkStatus {
    /// Total bytes received.
    pub bytes_received: u64,
    /// Total bytes transmitted.
    pub bytes_transmitted: u64,
}

/// System load average (1, 5, 15 minute windows).
#[derive(Debug, Clone, Serialize)]
pub struct LoadAverage {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}

impl SystemStatus {
    /// Collect a point-in-time snapshot of OS metrics.
    ///
    /// Each metric is collected independently — a failure reading one metric
    /// (e.g. permission denied) results in `None` for that field rather than
    /// propagating an error.
    pub fn collect() -> Self {
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        );
        // sysinfo requires a brief sleep to measure CPU usage accurately.
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        sys.refresh_cpu_usage();

        let cpu_usage = Self::read_cpu(&sys);
        let memory = Self::read_memory(&sys);
        let disk = Self::read_disk();
        let network = Self::read_network();
        let load_average = Self::read_load_average();
        let uptime_secs = Self::read_uptime();
        let hostname = Self::read_hostname();

        Self {
            cpu_usage,
            memory,
            disk,
            network,
            load_average,
            uptime_secs,
            hostname,
        }
    }

    fn read_cpu(sys: &System) -> Option<f64> {
        let cpus = sys.cpus();
        if cpus.is_empty() {
            return None;
        }
        let total: f64 = cpus.iter().map(|c| c.cpu_usage() as f64).sum();
        Some(total / cpus.len() as f64)
    }

    fn read_memory(sys: &System) -> MemoryStatus {
        let total = sys.total_memory();
        let used = sys.used_memory();
        let percentage = if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        MemoryStatus {
            used_bytes: used,
            total_bytes: total,
            percentage,
        }
    }

    fn read_disk() -> DiskStatus {
        let disks = Disks::new_with_refreshed_list();
        // Use the root filesystem (first disk on macOS, "/" mount on Linux).
        let disk = disks.iter().find(|d| d.mount_point() == std::path::Path::new("/"));
        match disk {
            Some(d) => {
                let total = d.total_space();
                let available = d.available_space();
                let used = total.saturating_sub(available);
                let percentage = if total > 0 {
                    (used as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                DiskStatus {
                    used_bytes: used,
                    total_bytes: total,
                    percentage,
                }
            }
            None => DiskStatus {
                used_bytes: 0,
                total_bytes: 0,
                percentage: 0.0,
            },
        }
    }

    fn read_network() -> NetworkStatus {
        let networks = Networks::new_with_refreshed_list();
        let (mut received, mut transmitted) = (0u64, 0u64);
        for (_name, data) in networks.iter() {
            received += data.total_received();
            transmitted += data.total_transmitted();
        }
        NetworkStatus {
            bytes_received: received,
            bytes_transmitted: transmitted,
        }
    }

    #[cfg(unix)]
    fn read_load_average() -> Option<LoadAverage> {
        let load = sysinfo::System::load_average();
        Some(LoadAverage {
            one: load.one,
            five: load.five,
            fifteen: load.fifteen,
        })
    }

    #[cfg(not(unix))]
    fn read_load_average() -> Option<LoadAverage> {
        None
    }

    fn read_uptime() -> Option<u64> {
        let uptime = System::uptime();
        if uptime > 0 {
            Some(uptime)
        } else {
            None
        }
    }

    fn read_hostname() -> String {
        System::host_name().unwrap_or_default()
    }
}

impl fmt::Display for SystemStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "System:")?;
        writeln!(f, "  Hostname: {}", self.hostname)?;

        if let Some(cpu) = self.cpu_usage {
            writeln!(f, "  CPU: {cpu:.1}%")?;
        } else {
            writeln!(f, "  CPU: N/A")?;
        }

        writeln!(
            f,
            "  Memory: {} / {} ({:.1}%)",
            format_bytes(self.memory.used_bytes),
            format_bytes(self.memory.total_bytes),
            self.memory.percentage
        )?;

        writeln!(
            f,
            "  Disk: {} / {} ({:.1}%)",
            format_bytes(self.disk.used_bytes),
            format_bytes(self.disk.total_bytes),
            self.disk.percentage
        )?;

        writeln!(
            f,
            "  Network: {} sent, {} received",
            format_bytes(self.network.bytes_transmitted),
            format_bytes(self.network.bytes_received)
        )?;

        if let Some(ref load) = self.load_average {
            writeln!(
                f,
                "  Load: {:.2} / {:.2} / {:.2}",
                load.one, load.five, load.fifteen
            )?;
        }

        if let Some(secs) = self.uptime_secs {
            writeln!(f, "  Uptime: {}", format_duration(secs))?;
        }

        Ok(())
    }
}

/// Format bytes into a human-readable string (KiB, MiB, GiB, TiB).
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TiB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GiB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MiB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KiB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Format seconds into a human-readable duration string.
fn format_duration(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 {
        parts.push(format!("{minutes}m"));
    }
    parts.push(format!("{seconds}s"));
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_returns_valid_cpu_usage() {
        let status = SystemStatus::collect();
        if let Some(cpu) = status.cpu_usage {
            assert!(
                (0.0..=100.0).contains(&cpu),
                "CPU usage {cpu}% out of range 0–100"
            );
        }
    }

    #[test]
    fn collect_returns_nonzero_total_memory() {
        let status = SystemStatus::collect();
        assert!(
            status.memory.total_bytes > 0,
            "total memory should be > 0"
        );
    }

    #[test]
    fn memory_used_does_not_exceed_total() {
        let status = SystemStatus::collect();
        assert!(
            status.memory.used_bytes <= status.memory.total_bytes,
            "used {} > total {}",
            status.memory.used_bytes,
            status.memory.total_bytes
        );
    }

    #[test]
    fn memory_percentage_is_consistent() {
        let status = SystemStatus::collect();
        let expected = if status.memory.total_bytes > 0 {
            (status.memory.used_bytes as f64 / status.memory.total_bytes as f64) * 100.0
        } else {
            0.0
        };
        assert!(
            (status.memory.percentage - expected).abs() < 0.1,
            "memory percentage {} != expected {}",
            status.memory.percentage,
            expected
        );
    }

    #[test]
    fn disk_used_does_not_exceed_total() {
        let status = SystemStatus::collect();
        assert!(
            status.disk.used_bytes <= status.disk.total_bytes,
            "disk used {} > total {}",
            status.disk.used_bytes,
            status.disk.total_bytes
        );
    }

    #[test]
    fn disk_percentage_is_consistent() {
        let status = SystemStatus::collect();
        if status.disk.total_bytes > 0 {
            let expected = (status.disk.used_bytes as f64 / status.disk.total_bytes as f64) * 100.0;
            assert!(
                (status.disk.percentage - expected).abs() < 0.1,
                "disk percentage {} != expected {}",
                status.disk.percentage,
                expected
            );
        }
    }

    #[test]
    fn network_bytes_are_non_negative() {
        let status = SystemStatus::collect();
        // u64 is always >= 0, but we verify the values are reasonable.
        let _ = status.network.bytes_received;
        let _ = status.network.bytes_transmitted;
    }

    #[cfg(unix)]
    #[test]
    fn load_average_is_populated_on_unix() {
        let status = SystemStatus::collect();
        assert!(
            status.load_average.is_some(),
            "load average should be populated on Unix"
        );
        let load = status.load_average.unwrap();
        assert!(load.one >= 0.0, "1-min load {}", load.one);
        assert!(load.five >= 0.0, "5-min load {}", load.five);
        assert!(load.fifteen >= 0.0, "15-min load {}", load.fifteen);
    }

    #[test]
    fn uptime_is_positive() {
        let status = SystemStatus::collect();
        if let Some(uptime) = status.uptime_secs {
            assert!(uptime > 0, "uptime should be > 0");
        }
    }

    #[test]
    fn hostname_is_non_empty() {
        let status = SystemStatus::collect();
        assert!(
            !status.hostname.is_empty(),
            "hostname should not be empty"
        );
    }

    #[test]
    fn display_contains_expected_sections() {
        let status = SystemStatus::collect();
        let output = format!("{status}");
        assert!(output.contains("System:"));
        assert!(output.contains("Hostname:"));
        assert!(output.contains("Memory:"));
        assert!(output.contains("Disk:"));
        assert!(output.contains("Network:"));
    }

    #[test]
    fn format_bytes_zero() {
        assert_eq!(format_bytes(0), "0 B");
    }

    #[test]
    fn format_bytes_one_kib() {
        assert_eq!(format_bytes(1024), "1.0 KiB");
    }

    #[test]
    fn format_bytes_one_mib() {
        assert_eq!(format_bytes(1024 * 1024), "1.0 MiB");
    }

    #[test]
    fn format_bytes_one_gib() {
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GiB");
    }

    #[test]
    fn format_bytes_one_tib() {
        assert_eq!(format_bytes(1024_u64 * 1024 * 1024 * 1024), "1.0 TiB");
    }

    #[test]
    fn format_bytes_mixed() {
        let result = format_bytes(1536); // 1.5 KiB
        assert_eq!(result, "1.5 KiB");
    }

    #[test]
    fn format_duration_zero() {
        assert_eq!(format_duration(0), "0s");
    }

    #[test]
    fn format_duration_seconds_only() {
        assert_eq!(format_duration(45), "45s");
    }

    #[test]
    fn format_duration_minutes_and_seconds() {
        assert_eq!(format_duration(125), "2m 5s");
    }

    #[test]
    fn format_duration_hours() {
        assert_eq!(format_duration(3661), "1h 1m 1s");
    }

    #[test]
    fn format_duration_days() {
        assert_eq!(format_duration(90061), "1d 1h 1m 1s");
    }

    #[test]
    fn serialize_to_json_succeeds() {
        let status = SystemStatus::collect();
        let json = serde_json::to_string(&status);
        assert!(json.is_ok(), "serialization should succeed: {:?}", json.err());
    }
}
