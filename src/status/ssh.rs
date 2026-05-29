//! SSH subsystem status: mux master, control path, config parse, agent, keys.
//!
//! Uses `ssh -O check` to probe the mux master, validates control paths via
//! `fs::symlink_metadata`, and shells out to `ssh-keygen -L` for config
//! parsing. Key counting uses `ssh-add -l`.
//!
//! # Control path validation
//!
//! The control path must satisfy **all** of:
//!
//! 1. Exist and be a Unix socket (or named pipe on Windows).
//! 2. Have permissions `0600` (owner read/write only).
//! 3. Be connectable (non-blocking `UnixStream::connect`).
//! 4. Have a valid, non-expired `CtlTimeMs` (if the mux supports it).

use std::fmt;

use serde::Serialize;

/// SSH subsystem status snapshot.
#[derive(Debug, Clone, Serialize)]
pub struct SshStatus {
    /// Whether the SSH mux master is alive.
    pub mux_master_alive: bool,
    /// Whether the control path is valid.
    pub control_path_valid: bool,
    /// Whether the SSH config parsed without errors.
    pub config_valid: bool,
    /// Whether the SSH agent is running.
    pub agent_running: bool,
    /// Number of keys loaded in the agent.
    pub key_count: u32,
}

impl SshStatus {
    /// Collect SSH subsystem status.
    ///
    /// Currently returns a placeholder. The actual implementation will probe
    /// the mux master, validate control paths, parse config, and count keys.
    pub fn collect() -> Self {
        Self {
            mux_master_alive: false,
            control_path_valid: false,
            config_valid: false,
            agent_running: false,
            key_count: 0,
        }
    }
}

impl fmt::Display for SshStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "SSH:")?;
        writeln!(
            f,
            "  Mux master: {}",
            if self.mux_master_alive { "alive" } else { "dead" }
        )?;
        writeln!(
            f,
            "  Control path: {}",
            if self.control_path_valid {
                "valid"
            } else {
                "invalid"
            }
        )?;
        writeln!(
            f,
            "  Config: {}",
            if self.config_valid { "ok" } else { "error" }
        )?;
        writeln!(
            f,
            "  Agent: {}",
            if self.agent_running { "running" } else { "stopped" }
        )?;
        writeln!(f, "  Keys: {}", self.key_count)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_returns_default_state() {
        let status = SshStatus::collect();
        assert!(!status.mux_master_alive);
        assert!(!status.control_path_valid);
        assert!(!status.config_valid);
        assert!(!status.agent_running);
        assert_eq!(status.key_count, 0);
    }

    #[test]
    fn display_contains_section_header() {
        let status = SshStatus::collect();
        let output = format!("{status}");
        assert!(output.contains("SSH:"));
        assert!(output.contains("Mux master:"));
        assert!(output.contains("Control path:"));
        assert!(output.contains("Config:"));
        assert!(output.contains("Agent:"));
        assert!(output.contains("Keys:"));
    }

    #[test]
    fn display_shows_alive_mux() {
        let status = SshStatus {
            mux_master_alive: true,
            control_path_valid: true,
            config_valid: true,
            agent_running: true,
            key_count: 3,
        };
        let output = format!("{status}");
        assert!(output.contains("Mux master: alive"));
        assert!(output.contains("Control path: valid"));
        assert!(output.contains("Config: ok"));
        assert!(output.contains("Agent: running"));
        assert!(output.contains("Keys: 3"));
    }

    #[test]
    fn display_shows_dead_mux() {
        let status = SshStatus::collect();
        let output = format!("{status}");
        assert!(output.contains("Mux master: dead"));
        assert!(output.contains("Control path: invalid"));
        assert!(output.contains("Config: error"));
        assert!(output.contains("Agent: stopped"));
    }

    #[test]
    fn serialize_to_json_succeeds() {
        let status = SshStatus::collect();
        let json = serde_json::to_string(&status);
        assert!(json.is_ok(), "serialization should succeed: {:?}", json.err());
    }
}
