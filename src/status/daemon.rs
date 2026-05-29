//! Daemon liveness and health checks.
//!
//! Reads PID files, checks `/proc` (Linux) or `kill -0` (macOS/Windows),
//! parses restart-count files, and detects stale Unix sockets.
//!
//! # Stale socket detection
//!
//! On Unix platforms, [`DaemonStatus::collect`] attempts to connect to the
//! daemon's Unix socket. If the connection is refused or times out, the
//! socket is flagged as stale so the caller can clean it up.

use std::fmt;

use serde::Serialize;

/// Daemon liveness and health snapshot.
#[derive(Debug, Clone, Serialize)]
pub struct DaemonStatus {
    /// Whether the daemon process is alive.
    pub alive: bool,
    /// Daemon PID, if known.
    pub pid: Option<u32>,
    /// Daemon uptime in seconds, if known.
    pub uptime_secs: Option<u64>,
    /// Number of daemon restarts since last clean start.
    pub restart_count: u32,
    /// Whether the daemon's Unix socket is stale (connection refused).
    pub stale_socket: bool,
}

impl DaemonStatus {
    /// Collect daemon status.
    ///
    /// Currently returns a placeholder. The actual implementation will read
    /// PID files, check process liveness, and probe Unix sockets.
    pub fn collect() -> Self {
        Self {
            alive: false,
            pid: None,
            uptime_secs: None,
            restart_count: 0,
            stale_socket: false,
        }
    }
}

impl fmt::Display for DaemonStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Daemon:")?;
        writeln!(f, "  Alive: {}", if self.alive { "yes" } else { "no" })?;
        if let Some(pid) = self.pid {
            writeln!(f, "  PID: {pid}")?;
        }
        if let Some(secs) = self.uptime_secs {
            writeln!(f, "  Uptime: {secs}s")?;
        }
        writeln!(f, "  Restarts: {}", self.restart_count)?;
        writeln!(
            f,
            "  Socket: {}",
            if self.stale_socket { "stale" } else { "ok" }
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_returns_default_state() {
        let status = DaemonStatus::collect();
        assert!(!status.alive, "default alive should be false");
        assert!(status.pid.is_none(), "default pid should be None");
        assert_eq!(status.restart_count, 0);
        assert!(!status.stale_socket);
    }

    #[test]
    fn display_contains_section_header() {
        let status = DaemonStatus::collect();
        let output = format!("{status}");
        assert!(output.contains("Daemon:"));
        assert!(output.contains("Alive:"));
        assert!(output.contains("Restarts:"));
        assert!(output.contains("Socket:"));
    }

    #[test]
    fn display_shows_pid_when_present() {
        let status = DaemonStatus {
            alive: true,
            pid: Some(12345),
            uptime_secs: Some(3600),
            restart_count: 2,
            stale_socket: false,
        };
        let output = format!("{status}");
        assert!(output.contains("PID: 12345"));
        assert!(output.contains("Uptime: 3600s"));
    }

    #[test]
    fn display_shows_stale_socket() {
        let status = DaemonStatus {
            alive: false,
            pid: None,
            uptime_secs: None,
            restart_count: 0,
            stale_socket: true,
        };
        let output = format!("{status}");
        assert!(output.contains("Socket: stale"));
    }

    #[test]
    fn serialize_to_json_succeeds() {
        let status = DaemonStatus::collect();
        let json = serde_json::to_string(&status);
        assert!(json.is_ok(), "serialization should succeed: {:?}", json.err());
    }
}
