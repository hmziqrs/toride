//! Unified error types for the `toride-cloud` crate.
//!
//! Every subsystem returns [`Error`] through the crate-level [`Result`] alias.
//! The enum is marked `#[non_exhaustive]` so new variants can be added without
//! a semver break.

// ---------------------------------------------------------------------------
// Error enum -- single source of truth for the entire crate
// ---------------------------------------------------------------------------

/// Crate-level error type covering all cloud provider subsystems.
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

    /// Required binary not found on `$PATH`.
    #[error("binary not found: {0}")]
    BinaryNotFound(String),

    /// An external command exited with a non-zero status.
    #[error("command `{program}` failed: {message}")]
    CommandFailed {
        /// The program that was invoked.
        program: String,
        /// Human-readable error message.
        message: String,
    },

    /// The cloud provider could not be detected or is not supported.
    #[error("cloud provider not found: {0}")]
    ProviderNotFound(String),

    /// A firewall rule conflicts with an existing rule.
    #[error("firewall rule conflict: {0}")]
    FirewallRuleConflict(String),

    /// A configuration file could not be parsed.
    #[error("config parse error: {0}")]
    ConfigParse(String),

    /// A catch-all error for cases that don't fit other variants.
    #[error("{0}")]
    Other(String),
}

// ---------------------------------------------------------------------------
// Crate-level result alias
// ---------------------------------------------------------------------------

/// Crate-level result alias used throughout the library.
pub type Result<T> = std::result::Result<T, Error>;
