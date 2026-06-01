//! Command-line interface for backup management.
//!
//! Provides clap-based argument parsing for the toride-backup CLI binary.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Backup CLI for toride.
#[derive(Parser, Debug)]
#[command(name = "toride-backup", about = "Backup scheduling and management")]
pub struct Cli {
    /// Path to configuration file.
    #[arg(
        short,
        long,
        default_value = "~/.config/toride/backup/config.json"
    )]
    pub config: PathBuf,

    /// Enable verbose logging.
    #[arg(short, long)]
    pub verbose: bool,

    /// Dry run mode - log actions without executing.
    #[arg(long)]
    pub dry_run: bool,

    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,
}

/// Available CLI subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run a backup job.
    Backup {
        /// Name of the backup job to run.
        name: String,
    },

    /// Run retention pruning for a backup job.
    Prune {
        /// Name of the backup job to prune.
        name: String,
    },

    /// Restore from a backup.
    Restore {
        /// Name of the backup job to restore from.
        name: String,
        /// Target directory for the restore.
        #[arg(short, long)]
        target: PathBuf,
        /// Specific snapshot ID (defaults to latest).
        #[arg(short, long)]
        snapshot: Option<String>,
        /// Specific paths to restore (empty = full restore).
        #[arg(short, long)]
        paths: Option<Vec<String>>,
    },

    /// Run a test restore to verify backup integrity.
    TestRestore {
        /// Name of the backup job to test.
        name: String,
    },

    /// List snapshots in a repository.
    Snapshots {
        /// Name of the backup job.
        name: String,
    },

    /// Run diagnostic checks.
    Doctor {
        /// Specific check category to run (defaults to all).
        #[arg(short, long)]
        scope: Option<String>,
    },

    /// Install a backup schedule.
    InstallSchedule {
        /// Name of the backup job.
        name: String,
    },

    /// Remove a backup schedule.
    RemoveSchedule {
        /// Name of the backup job.
        name: String,
    },

    /// Show backup configuration and status.
    Status {
        /// Name of a specific job (omit for all jobs).
        name: Option<String>,
    },

    /// Validate configuration without running backups.
    Validate,
}
