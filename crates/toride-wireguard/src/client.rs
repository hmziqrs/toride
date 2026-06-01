//! WireGuard client wrapping `wg` CLI commands.
//!
//! [`WireguardClient`] provides methods for interacting with the WireGuard
//! kernel module via the `wg` CLI tool. It handles command execution, output
//! parsing, and error translation.

use crate::error::Result;
use crate::parse::WgShowEntry;

// ---------------------------------------------------------------------------
// WireguardClient
// ---------------------------------------------------------------------------

/// Client for interacting with WireGuard via the `wg` CLI.
///
/// Wraps `wg show`, `wg showconf`, and `wg set` commands with proper error
/// handling and output parsing.
///
/// # Construction
///
/// - [`WireguardClient::new`] -- production defaults using `duct`.
/// - [`WireguardClient::with_runner`] -- inject a custom command runner.
pub struct WireguardClient {
    _runner: (),
}

impl WireguardClient {
    /// Create a new client using the default command runner.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BinaryNotFound`] if `wg` is not on `$PATH`.
    pub fn new() -> Result<Self> {
        // TODO: verify `wg` binary is available via `which`.
        tracing::debug!("creating WireguardClient");
        Ok(Self { _runner: () })
    }

    /// Create a client with a custom command runner (for testing).
    pub fn with_runner(_runner: ()) -> Self {
        Self { _runner: () }
    }

    /// Show all WireGuard interfaces.
    ///
    /// Executes `wg show` and parses the output.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the command fails.
    pub fn show(&self) -> Result<Vec<WgShowEntry>> {
        tracing::debug!("running `wg show`");
        // TODO: execute `wg show` and parse output.
        Ok(Vec::new())
    }

    /// Show the configuration for a specific interface.
    ///
    /// Executes `wg showconf <interface>`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InterfaceNotFound`] if the interface does not exist,
    /// or [`Error::CommandFailed`] if the command fails.
    pub fn showconf(&self, interface: &str) -> Result<String> {
        tracing::debug!("running `wg showconf {interface}`");
        // TODO: execute `wg showconf <interface>`.
        Ok(String::new())
    }

    /// Set a configuration on a running interface.
    ///
    /// Executes `wg setconf <interface> <config>`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the command fails.
    pub fn setconf(&self, interface: &str, config: &str) -> Result<()> {
        tracing::debug!("running `wg setconf {interface}`");
        // TODO: execute `wg setconf <interface> <config>`.
        Ok(())
    }

    /// Sync a configuration file to a running interface.
    ///
    /// Executes `wg syncconf <interface> <path>`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the command fails.
    pub fn syncconf(&self, interface: &str, config_path: &str) -> Result<()> {
        tracing::debug!("running `wg syncconf {interface} {config_path}`");
        // TODO: execute `wg syncconf <interface> <path>`.
        Ok(())
    }
}

impl Default for WireguardClient {
    fn default() -> Self {
        Self::new().expect("WireguardClient::new() failed")
    }
}
