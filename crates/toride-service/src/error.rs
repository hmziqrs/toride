//! Unified error types for the `toride-service` crate.
//!
//! Every subsystem returns [`Error`] through the crate-level [`Result`] alias.
//! The enum is marked `#[non_exhaustive]` so new variants can be added without
//! a semver break.

// ---------------------------------------------------------------------------
// Error enum -- single source of truth for the entire crate
// ---------------------------------------------------------------------------

/// Crate-level error type covering all service management operations.
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

    /// The requested service unit was not found on the system.
    #[error("service not found: {0}")]
    ServiceNotFound(String),

    /// The service entered a failed state.
    #[error("service failed: {0}")]
    ServiceFailed(String),

    /// An external command (e.g. `systemctl`) exited with a non-zero status.
    #[error("command failed: {0}")]
    CommandFailed(String),

    /// The service is not installed on the system.
    #[error("service not installed: {0}")]
    NotInstalled(String),

    /// A generic catch-all error for cases that do not fit a specific variant.
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
