//! CLI argument definitions via clap.
//!
//! Provides the command-line interface for the `toride-users` binary
//! or integration with the main `toride` CLI.

use clap::{Parser, Subcommand};

/// Toride users management CLI.
#[derive(Debug, Parser)]
#[command(name = "toride-users", version, about = "OS-level user and access control management")]
pub struct Cli {
    /// The subcommand to run.
    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Create a new user account.
    Create {
        /// Username for the new account.
        username: String,
        /// Login shell.
        #[arg(long, default_value = "/usr/bin/bash")]
        shell: String,
        /// Supplementary groups (comma-separated).
        #[arg(long, value_delimiter = ',')]
        groups: Vec<String>,
        /// Grant sudo access.
        #[arg(long)]
        sudo: bool,
        /// Enable TOTP/2FA.
        #[arg(long)]
        totp: bool,
    },

    /// Delete a user account.
    Delete {
        /// Username to delete.
        username: String,
        /// Remove the home directory.
        #[arg(long)]
        remove_home: bool,
    },

    /// Grant sudo access to a user.
    SudoGrant {
        /// Username.
        username: String,
        /// Grant passwordless sudo (NOPASSWD).
        #[arg(long)]
        nopasswd: bool,
    },

    /// Revoke sudo access from a user.
    SudoRevoke {
        /// Username.
        username: String,
    },

    /// Enroll a user in TOTP/2FA.
    TotpEnroll {
        /// Username.
        username: String,
    },

    /// Remove TOTP/2FA for a user.
    TotpRemove {
        /// Username.
        username: String,
    },

    /// Lock a user account.
    Lock {
        /// Username.
        username: String,
    },

    /// Unlock a user account.
    Unlock {
        /// Username.
        username: String,
    },

    /// Run diagnostic checks.
    Doctor {
        /// Scope of checks to run.
        #[arg(long, default = "all")]
        scope: String,
    },

    /// Show user information.
    Info {
        /// Username to inspect.
        username: String,
    },
}

/// Parse CLI arguments from strings (useful for testing).
///
/// # Errors
///
/// Returns an error if the arguments cannot be parsed.
pub fn parse_args<I, S>(args: I) -> clap::Result<Cli>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    Cli::try_parse_from(args)
}
