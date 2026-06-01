//! Unified error types for the `toride-audit` crate.
//!
//! Every subsystem returns [`Error`] through the crate-level [`Result`] alias.
//! The enum is marked `#[non_exhaustive]` so new variants can be added without
//! a semver break.

// ---------------------------------------------------------------------------
// Error enum -- single source of truth for the entire crate
// ---------------------------------------------------------------------------

/// Crate-level error type covering all audit subsystems.
///
/// Uses [`thiserror`] for `Display` and `std::error::Error` impls.
/// Marked `#[non_exhaustive]` so downstream crates must handle future
/// variants with a wildcard match arm.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An I/O error propagated from `std::io`.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A required binary was not found on `$PATH`.
    #[error("binary not found: {0}")]
    BinaryNotFound(String),

    /// An external command exited with a non-zero status.
    #[error("command failed: {0}")]
    CommandFailed(String),

    /// An audit rule could not be parsed.
    #[error("audit rule parse error: {0}")]
    AuditRuleParse(String),

    /// An AIDE (file integrity) error occurred.
    #[error("AIDE error: {0}")]
    AideError(String),

    /// A log rotation operation failed.
    #[error("log rotation error: {0}")]
    LogRotateError(String),

    /// A configuration file could not be parsed.
    #[error("config parse error: {0}")]
    ConfigParse(String),

    /// A configuration file could not be written.
    #[error("config write error: {0}")]
    ConfigWrite(String),

    /// A generic or unclassified error.
    #[error("{0}")]
    Other(String),
}

impl From<toride_runner::Error> for Error {
    fn from(err: toride_runner::Error) -> Self {
        Self::CommandFailed(err.to_string())
    }
}

// ---------------------------------------------------------------------------
// Crate-level result alias
// ---------------------------------------------------------------------------

/// Crate-level result alias used throughout the library.
pub type Result<T> = std::result::Result<T, Error>;
