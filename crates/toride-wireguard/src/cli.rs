//! Clap argument definitions for the WireGuard CLI.
//!
//! Provides structured argument parsing for WireGuard management commands
//! using the `clap` derive API.

use clap::{Parser, Subcommand};

// ---------------------------------------------------------------------------
// WireguardCli
// ---------------------------------------------------------------------------

/// WireGuard tunnel management CLI.
#[derive(Debug, Parser)]
#[command(name = "wireguard", about = "WireGuard VPN tunnel management")]
pub struct WireguardCli {
    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: WireguardCommand,

    /// Interface name (e.g. `wg0`).
    #[arg(short, long, global = true, default_value = "wg0")]
    pub interface: String,
}

// ---------------------------------------------------------------------------
// WireguardCommand
// ---------------------------------------------------------------------------

/// Available WireGuard management commands.
#[derive(Debug, Subcommand)]
pub enum WireguardCommand {
    /// Show interface status and peer information.
    Show,

    /// Bring up a WireGuard interface.
    Up,

    /// Bring down a WireGuard interface.
    Down,

    /// Generate a new key pair.
    Genkey,

    /// Run diagnostic checks.
    Doctor {
        /// Scope of diagnostics to run.
        #[arg(short, long, default_value = "all")]
        scope: String,
    },

    /// Manage interface configuration.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

// ---------------------------------------------------------------------------
// ConfigAction
// ---------------------------------------------------------------------------

/// Configuration subcommands.
#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Show the current configuration.
    Show,

    /// Apply a new configuration from a file.
    Apply {
        /// Path to the configuration file.
        path: String,

        /// Preview changes before applying.
        #[arg(long)]
        dry_run: bool,
    },

    /// Backup the current configuration.
    Backup,

    /// Restore the most recent backup.
    Restore,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parse_show_command() {
        let cli = WireguardCli::try_parse_from(["wireguard", "show"]).unwrap();
        assert!(matches!(cli.command, WireguardCommand::Show));
        assert_eq!(cli.interface, "wg0");
    }

    #[test]
    fn parse_up_with_interface() {
        let cli = WireguardCli::try_parse_from(["wireguard", "-i", "wg1", "up"]).unwrap();
        assert!(matches!(cli.command, WireguardCommand::Up));
        assert_eq!(cli.interface, "wg1");
    }

    #[test]
    fn parse_genkey() {
        let cli = WireguardCli::try_parse_from(["wireguard", "genkey"]).unwrap();
        assert!(matches!(cli.command, WireguardCommand::Genkey));
    }

    #[test]
    fn parse_doctor() {
        let cli = WireguardCli::try_parse_from(["wireguard", "doctor", "--scope", "security"])
            .unwrap();
        assert!(matches!(cli.command, WireguardCommand::Doctor { .. }));
    }

    #[test]
    fn parse_config_show() {
        let cli =
            WireguardCli::try_parse_from(["wireguard", "config", "show"]).unwrap();
        assert!(matches!(
            cli.command,
            WireguardCommand::Config {
                action: ConfigAction::Show
            }
        ));
    }

    #[test]
    fn parse_config_apply_dry_run() {
        let cli = WireguardCli::try_parse_from([
            "wireguard",
            "config",
            "apply",
            "--dry-run",
            "/tmp/wg0.conf",
        ])
        .unwrap();
        if let WireguardCommand::Config {
            action: ConfigAction::Apply { path, dry_run },
        } = cli.command
        {
            assert_eq!(path, "/tmp/wg0.conf");
            assert!(dry_run);
        } else {
            panic!("expected ConfigAction::Apply");
        }
    }
}
