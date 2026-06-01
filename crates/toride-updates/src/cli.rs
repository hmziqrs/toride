//! CLI argument parsing for the updates command.
//!
//! Uses [`clap`] to define the command-line interface for managing automatic
//! security updates. This module is gated behind the `cli` feature.

// ---------------------------------------------------------------------------
// CLI types
// ---------------------------------------------------------------------------

/// Top-level CLI arguments for the `toride updates` subcommand.
#[derive(Debug, Clone, clap::Parser)]
#[command(name = "updates", about = "Manage automatic security updates")]
pub struct UpdatesCli {
    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: UpdatesCommand,
}

/// Subcommands for the updates CLI.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum UpdatesCommand {
    /// Show the current update status.
    Status,

    /// Configure automatic updates.
    Configure(ConfigureArgs),

    /// Check for available updates without applying them.
    Check,

    /// Apply pending updates now.
    Apply,

    /// Run diagnostic checks on the update subsystem.
    Doctor,

    /// Show the update schedule.
    Schedule(ScheduleArgs),
}

/// Arguments for the `configure` subcommand.
#[derive(Debug, Clone, clap::Parser)]
#[command(about = "Configure automatic update settings")]
pub struct ConfigureArgs {
    /// Enable or disable automatic updates.
    #[arg(long, action = clap::ArgAction::Set)]
    pub auto_update: Option<bool>,

    /// Only install security updates (skip feature/bugfix updates).
    #[arg(long, action = clap::ArgAction::Set)]
    pub security_only: Option<bool>,

    /// Set the reboot policy after updates.
    #[arg(long, value_enum)]
    pub reboot: Option<RebootPolicyArg>,

    /// Add an APT origin pattern for update selection.
    #[arg(long, action = clap::ArgAction::Append)]
    pub origin: Option<Vec<String>>,
}

/// Arguments for the `schedule` subcommand.
#[derive(Debug, Clone, clap::Parser)]
#[command(about = "Manage the automatic update schedule")]
pub struct ScheduleArgs {
    /// Set the update frequency.
    #[arg(long, value_enum)]
    pub set: Option<ScheduleArg>,

    /// Set a custom systemd calendar expression.
    #[arg(long)]
    pub custom: Option<String>,

    /// Remove the current schedule.
    #[arg(long)]
    pub remove: bool,
}

/// CLI argument for schedule frequency.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum ScheduleArg {
    /// Run once per day.
    Daily,
    /// Run once per week.
    Weekly,
    /// Run once per month.
    Monthly,
}

/// CLI argument for reboot policy.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum RebootPolicyArg {
    /// Never reboot automatically.
    Never,
    /// Reboot only when required by an updated package.
    WhenNeeded,
    /// Always reboot after applying updates.
    Always,
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

impl From<ScheduleArg> for crate::spec::Schedule {
    fn from(val: ScheduleArg) -> Self {
        match val {
            ScheduleArg::Daily => Self::Daily,
            ScheduleArg::Weekly => Self::Weekly,
            ScheduleArg::Monthly => Self::Monthly,
        }
    }
}

impl From<RebootPolicyArg> for crate::spec::RebootPolicy {
    fn from(val: RebootPolicyArg) -> Self {
        match val {
            RebootPolicyArg::Never => Self::Never,
            RebootPolicyArg::WhenNeeded => Self::WhenNeeded,
            RebootPolicyArg::Always => Self::Always,
        }
    }
}
