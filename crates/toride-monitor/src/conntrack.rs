//! Conntrack data parsing and connection tracking.
//!
//! Provides [`ConntrackReader`] for querying the kernel connection tracking
//! table via the `conntrack` command and converting raw output into
//! structured types.

use crate::parse::{parse_conntrack_output, ConntrackEntry};
use crate::paths::MonitorPaths;
use crate::report::ConnectionInfo;
use crate::{Error, Result};

/// Reads connection tracking data from the kernel via `conntrack`.
///
/// Wraps the `conntrack` command-line tool to list, filter, and parse
/// connection tracking entries.
pub struct ConntrackReader<'a> {
    /// Binary paths for system commands.
    paths: &'a MonitorPaths,
}

impl<'a> ConntrackReader<'a> {
    /// Create a new `ConntrackReader` with the given paths.
    #[must_use]
    pub fn new(paths: &'a MonitorPaths) -> Self {
        Self { paths }
    }

    /// List all connection tracking entries.
    ///
    /// Runs `conntrack -L` and parses the output into structured entries.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the `conntrack` command fails,
    /// or [`Error::ConntrackError`] if parsing fails.
    #[cfg(feature = "client")]
    pub fn list_all(&self) -> Result<Vec<ConntrackEntry>> {
        let output = duct::cmd(&self.paths.conntrack, ["-L"])
            .stderr_to_stdout()
            .stdout_capture()
            .run()
            .map_err(|e| Error::CommandFailed(format!("conntrack: {e}")))?;

        if !output.status.success() {
            return Err(Error::ConntrackError(format!(
                "conntrack -L failed: {}",
                String::from_utf8_lossy(&output.stdout)
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_conntrack_output(&stdout)
    }

    /// List connection tracking entries filtered by destination port.
    ///
    /// Runs `conntrack -L -p <proto> --dport <port>`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the `conntrack` command fails.
    #[cfg(feature = "client")]
    pub fn list_by_dport(&self, proto: &str, port: u16) -> Result<Vec<ConntrackEntry>> {
        let output = duct::cmd(
            &self.paths.conntrack,
            ["-L", "-p", proto, "--dport", &port.to_string()],
        )
        .stderr_to_stdout()
        .stdout_capture()
        .run()
        .map_err(|e| Error::CommandFailed(format!("conntrack: {e}")))?;

        if !output.status.success() {
            return Err(Error::ConntrackError(format!(
                "conntrack filter failed: {}",
                String::from_utf8_lossy(&output.stdout)
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_conntrack_output(&stdout)
    }

    /// Convert conntrack entries to [`ConnectionInfo`] instances.
    ///
    /// Maps raw conntrack data into the unified connection info type
    /// used by the reporting subsystem. This method does not require
    /// the `client` feature as it operates purely on in-memory data.
    pub fn to_connection_info(entries: &[ConntrackEntry]) -> Vec<ConnectionInfo> {
        entries
            .iter()
            .map(|e| ConnectionInfo {
                src: e.src,
                src_port: e.sport.unwrap_or(0),
                dst: e.dst,
                dst_port: e.dport.unwrap_or(0),
                protocol: match e.proto {
                    6 => "tcp".to_owned(),
                    17 => "udp".to_owned(),
                    1 => "icmp".to_owned(),
                    other => format!("proto-{other}"),
                },
                state: e.state.clone().unwrap_or_default(),
                bytes: e.bytes,
                packets: e.packets,
            })
            .collect()
    }

    /// Count the number of currently tracked connections.
    ///
    /// Runs `conntrack -C` to get the count directly.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the `conntrack` command fails.
    #[cfg(feature = "client")]
    pub fn count(&self) -> Result<u64> {
        let output = duct::cmd(&self.paths.conntrack, ["-C"])
            .stdout_capture()
            .run()
            .map_err(|e| Error::CommandFailed(format!("conntrack: {e}")))?;

        if !output.status.success() {
            return Err(Error::ConntrackError("conntrack -C failed".into()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let count: u64 = stdout
            .trim()
            .parse()
            .map_err(|e| Error::ConntrackError(format!("invalid count: {e}")))?;

        Ok(count)
    }
}
