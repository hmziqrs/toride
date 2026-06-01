//! `toride-updates` — automatic security update management for Linux VPS hosts.
//!
//! Wraps `unattended-upgrades` (Debian/Ubuntu) and `dnf-automatic` (Fedora/RHEL)
//! behind a unified API for configuring, scheduling, and monitoring automatic
//! security updates.
//!
//! # Architecture
//!
//! The crate is organized into several subsystems:
//!
//! - **[`spec`]** -- declarative update specification (`UpdateSpec`)
//! - **[`detect`]** -- package manager detection (`apt` vs `dnf`)
//! - **[`parse`]** -- parsing command output into structured types
//! - **[`render`]** -- rendering specs into config file content
//! - **[`validate`]** -- validating specs and configurations
//! - **[`paths`]** -- resolved filesystem paths for update configs
//! - **[`report`]** -- structured status and diagnostic reports
//! - **[`backup`]** -- pre-mutation config backup
//!
//! Feature-gated subsystems:
//!
//! - **`client`** -- command execution for update operations
//! - **`service`** -- systemd service management
//! - **`doctor`** -- diagnostic health checks
//! - **`config`** -- config file read/write
//! - **`apt`** -- Debian/Ubuntu-specific backend
//! - **`dnf`** -- Fedora/RHEL-specific backend
//! - **`schedule`** -- systemd timer / cron schedule management
//! - **`cli`** -- command-line argument parsing

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![expect(dead_code, reason = "scaffolding for modules under active development")]
#![expect(clippy::must_use_candidate, reason = "constructors and getters are obvious")]
#![expect(clippy::missing_errors_doc, reason = "library is internal")]

// ---------------------------------------------------------------------------
// Module declarations -- always compiled
// ---------------------------------------------------------------------------

/// Error types for the crate.
pub mod error;
/// Resolved filesystem paths for update configuration files.
pub mod paths;
/// Declarative update specification types.
pub mod spec;
/// Structured status reports and diagnostic findings.
pub mod report;
/// Parsers for command output (unattended-upgrades, apt-check, dnf).
pub mod parse;
/// Config file renderers (auto-upgrades, dnf-automatic, apt).
pub mod render;
/// Spec and configuration validation.
pub mod validate;
/// Pre-mutation config file backup.
pub mod backup;
/// Package manager detection (apt vs dnf).
pub mod detect;

// ---------------------------------------------------------------------------
// Module declarations -- feature-gated
// ---------------------------------------------------------------------------

/// Client for executing update commands.
#[cfg(feature = "client")]
pub mod client;

/// Service management for unattended-upgrades / dnf-automatic.
#[cfg(feature = "service")]
pub mod service;

/// Doctor checks for update subsystem health.
#[cfg(feature = "doctor")]
pub mod doctor;

/// Config file read/write with atomic writes.
#[cfg(feature = "config")]
pub mod config;

/// Debian/Ubuntu (APT) specific update backend.
#[cfg(feature = "apt")]
pub mod apt;

/// Fedora/RHEL (DNF) specific update backend.
#[cfg(feature = "dnf")]
pub mod dnf;

/// Systemd timer / cron schedule management.
#[cfg(feature = "schedule")]
pub mod schedule;

/// CLI argument parsing via clap.
#[cfg(feature = "cli")]
pub mod cli;

// ---------------------------------------------------------------------------
// Re-exports
// ---------------------------------------------------------------------------

pub use error::{Error, Result};
pub use spec::UpdateSpec;
