//! CLI argument definitions for the toride-monitor binary.
//!
//! Uses [`clap`] derive macros to define the command-line interface.

use clap::Parser;

/// Outbound traffic monitoring and anomaly detection.
#[derive(Debug, Parser)]
#[command(name = "toride-monitor", version, about)]
pub enum Cli {
    /// Set up iptables OUTPUT chain logging rules.
    Setup {
        /// Config file path (default: XDG config location).
        #[arg(short, long)]
        config: Option<String>,

        /// Dry run: print commands without executing.
        #[arg(long)]
        dry_run: bool,
    },

    /// Remove all iptables OUTPUT chain logging rules.
    Teardown {
        /// Dry run: print commands without executing.
        #[arg(long)]
        dry_run: bool,
    },

    /// Take a snapshot of current outbound connections.
    Snapshot {
        /// Output format.
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Run anomaly detection on the current traffic.
    Detect {
        /// Config file path with thresholds.
        #[arg(short, long)]
        config: Option<String>,

        /// Output format.
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Run diagnostic checks.
    Doctor {
        /// Scope of checks to run.
        #[arg(short, long, default_value = "all")]
        scope: String,
    },

    /// Run a single monitoring cycle (for daemon mode).
    Run {
        /// Config file path.
        #[arg(short, long)]
        config: Option<String>,
    },
}

/// Parse CLI arguments from the process environment.
///
/// Convenience wrapper around [`Cli::parse`].
pub fn parse_args() -> Cli {
    Cli::parse()
}
