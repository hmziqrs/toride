//! Command-line interface types for Tailscale operations.
//!
//! Provides [`TailscaleArgs`] as the top-level clap argument parser for
//! Tailscale subcommands, and [`TailscaleCommand`] for subcommand dispatch.

// ---------------------------------------------------------------------------
// TailscaleArgs
// ---------------------------------------------------------------------------

/// Top-level CLI arguments for Tailscale operations.
///
/// # Example
///
/// ```ignore
//! use clap::Parser;
//! use toride_tailscale::cli::TailscaleArgs;
//!
//! let args = TailscaleArgs::parse_from(["tailscale", "status"]);
/// // dispatch based on args.command
/// ```
#[derive(Debug, clap::Parser)]
#[command(name = "tailscale", about = "Tailscale mesh VPN management")]
pub struct TailscaleArgs {
    /// The subcommand to run.
    #[command(subcommand)]
    pub command: TailscaleCommand,

    /// Enable verbose logging.
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Run in dry-run mode (no mutations).
    #[arg(long, global = true)]
    pub dry_run: bool,
}

// ---------------------------------------------------------------------------
// TailscaleCommand
// ---------------------------------------------------------------------------

/// Available Tailscale subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum TailscaleCommand {
    /// Show the current connection status.
    Status,

    /// Run diagnostic checks.
    Doctor {
        /// Specific check to run (default: all).
        #[arg(long)]
        check: Option<String>,
    },

    /// Show network topology and peers.
    Peers,

    /// Run a network connectivity check.
    Netcheck,

    /// Show DNS configuration.
    Dns,

    /// Manage ACL policies.
    Acl {
        /// ACL subcommand.
        #[command(subcommand)]
        action: AclAction,
    },

    /// Manage the tailscaled service.
    Service {
        /// Service action: start, stop, restart, status.
        action: String,
    },
}

// ---------------------------------------------------------------------------
// AclAction
// ---------------------------------------------------------------------------

/// ACL management subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum AclAction {
    /// Validate the current ACL policy.
    Validate,

    /// Show the current ACL rules.
    Show,

    /// Apply a new ACL policy from a file.
    Apply {
        /// Path to the ACL policy file.
        path: String,
    },
}
