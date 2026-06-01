//! CLI argument definitions for the audit subsystem.
//!
//! Provides [`clap`] derive-based argument types for building CLI tools
//! that manage audit rules, file integrity, and log aggregation.

// ---------------------------------------------------------------------------
// AuditCli
// ---------------------------------------------------------------------------

/// Top-level CLI arguments for the audit tool.
#[derive(Debug, clap::Parser)]
#[command(name = "toride-audit", about = "Linux audit management", version)]
pub struct AuditCli {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: AuditCommand,

    /// Enable verbose output.
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Enable dry-run mode (log commands without executing).
    #[arg(long, global = true)]
    pub dry_run: bool,
}

// ---------------------------------------------------------------------------
// AuditCommand
// ---------------------------------------------------------------------------

/// Available audit subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum AuditCommand {
    /// Run diagnostic checks on the audit subsystem.
    Doctor {
        /// Scope of checks to run.
        #[arg(default_value = "all")]
        scope: String,
    },

    /// Manage audit rules.
    Rules {
        /// Subcommand for rule operations.
        #[command(subcommand)]
        action: RuleAction,
    },

    /// Manage file integrity monitoring (AIDE).
    Integrity {
        /// Subcommand for integrity operations.
        #[command(subcommand)]
        action: IntegrityAction,
    },

    /// Manage system logs.
    Logs {
        /// Subcommand for log operations.
        #[command(subcommand)]
        action: LogAction,
    },

    /// Manage the audit daemon.
    Daemon {
        /// Subcommand for daemon operations.
        #[command(subcommand)]
        action: DaemonAction,
    },
}

// ---------------------------------------------------------------------------
// RuleAction
// ---------------------------------------------------------------------------

/// Audit rule subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum RuleAction {
    /// List current audit rules.
    List,

    /// Apply a preset set of audit rules.
    Apply {
        /// Preset name (cis-level2, stig, minimal).
        preset: String,
    },

    /// Show diff between current and proposed rules.
    Diff {
        /// Preset name to compare against.
        preset: String,
    },
}

// ---------------------------------------------------------------------------
// IntegrityAction
// ---------------------------------------------------------------------------

/// Integrity monitoring subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum IntegrityAction {
    /// Initialize the AIDE database.
    Init,

    /// Run an integrity check.
    Check,

    /// Update the AIDE database after changes.
    Update,

    /// Show integrity status.
    Status,
}

// ---------------------------------------------------------------------------
// LogAction
// ---------------------------------------------------------------------------

/// Log management subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum LogAction {
    /// List managed log files.
    List,

    /// Show log storage usage.
    Usage,

    /// Vacuum old journal entries.
    Vacuum {
        /// Time specification (e.g. "7d", "2weeks").
        time: String,
    },
}

// ---------------------------------------------------------------------------
// DaemonAction
// ---------------------------------------------------------------------------

/// Audit daemon subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum DaemonAction {
    /// Start the audit daemon.
    Start,

    /// Stop the audit daemon.
    Stop,

    /// Restart the audit daemon.
    Restart,

    /// Show audit daemon status.
    Status,

    /// Reload audit rules without restarting.
    Reload,
}
