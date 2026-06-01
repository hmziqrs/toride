//! Unified error types for the `toride-fs` crate.
//!
//! Every subsystem returns [`Error`] through the crate-level [`Result`] alias.
//! The enum is marked `#[non_exhaustive]` so new variants can be added without
//! a semver break.

// ---------------------------------------------------------------------------
// Error enum -- single source of truth for the entire crate
// ---------------------------------------------------------------------------

/// Crate-level error type covering all filesystem subsystems.
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

    /// A path is invalid or could not be resolved.
    #[error("invalid path: {0}")]
    PathInvalid(String),

    /// Advisory file lock could not be acquired for write coordination.
    #[error("lock error: {0}")]
    LockFailed(String),

    /// A permission check failed (file mode, ownership, or OS capability).
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// An atomic write operation failed (temp file creation or persist).
    #[error("atomic write failed for `{path}`: {reason}")]
    AtomicWriteFailed {
        /// The target file path.
        path: String,
        /// Description of why the atomic write failed.
        reason: String,
    },
}

// ---------------------------------------------------------------------------
// Crate-level result alias
// ---------------------------------------------------------------------------

/// Crate-level result alias used throughout the library.
pub type Result<T> = std::result::Result<T, Error>;
