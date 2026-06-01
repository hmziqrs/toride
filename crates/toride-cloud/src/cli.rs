//! Command-line interface for toride-cloud.
//!
//! Defines the CLI argument structure using clap. Only compiled when the
//! `cli` feature is enabled.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Cloud provider security group management CLI for toride.
#[derive(Parser, Debug)]
#[command(name = "toride-cloud", about = "Cloud provider security group and firewall management")]
pub struct Cli {
    /// Path to configuration file.
    #[arg(short, long, default_value = "~/.config/toride/cloud/config.json")]
    pub config: PathBuf,

    /// Enable verbose logging.
    #[arg(short, long)]
    pub verbose: bool,

    /// Cloud provider to use (overrides auto-detection).
    #[arg(short, long)]
    pub provider: Option<String>,

    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    /// Resolve the `--provider` flag into a [`crate::CloudProvider`].
    pub fn resolve_provider(&self) -> crate::CloudProvider {
        match &self.provider {
            Some(p) => crate::CloudProvider::from_str_loose(p),
            None => crate::CloudProvider::Unknown,
        }
    }
}

/// Available CLI subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Detect the current cloud provider.
    Detect,

    /// List all security groups / firewalls.
    List {
        /// Output format (table, json).
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Show details of a specific security group.
    Show {
        /// Security group name or ID.
        name: String,
    },

    /// Run diagnostic checks.
    Doctor {
        /// Scope of checks (all, binaries, security-groups, agent).
        #[arg(short, long, default_value = "all")]
        scope: String,
    },

    /// Render firewall rules in human-readable format.
    Render {
        /// Security group name or ID (omit for all).
        name: Option<String>,
    },

    /// Validate firewall rules without applying changes.
    Validate {
        /// Security group name or ID (omit for all).
        name: Option<String>,
    },
}
