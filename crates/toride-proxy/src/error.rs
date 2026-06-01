//! Unified error types for the `toride-proxy` crate.
//!
//! Every subsystem returns [`Error`] through the crate-level [`Result`] alias.
//! The enum is marked `#[non_exhaustive]` so new variants can be added without
//! a semver break.
//!
//! # Variant naming convention
//!
//! Simple string-based errors use tuple variants (e.g. `NginxSyntax(String)`).
//! Structured errors that carry multiple fields use struct variants
//! (e.g. `Command { program, code, stderr }`).

// ---------------------------------------------------------------------------
// Error enum -- single source of truth for the entire crate
// ---------------------------------------------------------------------------

/// Crate-level error type covering all proxy subsystems.
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

    /// An error from the `toride-fs` crate.
    #[error("filesystem error: {0}")]
    Fs(#[from] toride_fs::Error),

    /// An error from the `toride-runner` crate.
    #[error("runner error: {0}")]
    Runner(#[from] toride_runner::error::Error),

    /// A required binary was not found on `$PATH`.
    #[error("binary not found: {0}")]
    BinaryNotFound(String),

    /// An external command exited with a non-zero status.
    #[error("command failed: {program} exited with code {code:?}")]
    CommandFailed {
        /// The program that was invoked.
        program: String,
        /// Exit code, or `None` if the process was killed.
        code: Option<i32>,
        /// Captured standard error (may be empty).
        stderr: String,
    },

    /// Nginx configuration syntax error.
    #[error("nginx syntax error: {0}")]
    NginxSyntax(String),

    /// Configuration file parse error.
    #[error("config parse error: {0}")]
    ConfigParse(String),

    /// Configuration file write error.
    #[error("config write error: {0}")]
    ConfigWrite(String),

    /// TLS certificate has expired or is not yet valid.
    #[error("certificate expired: {0}")]
    CertExpired(String),

    /// TLS certificate renewal failed.
    #[error("certificate renewal failed: {0}")]
    CertRenewal(String),

    /// Validation error for a spec or input value.
    #[error("validation error: {0}")]
    Validation(String),

    /// A requested resource was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Catch-all for errors that don't fit a specific variant.
    #[error("{0}")]
    Other(String),
}

// ---------------------------------------------------------------------------
// Crate-level result alias
// ---------------------------------------------------------------------------

/// Crate-level result alias used throughout the library.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let e = Error::BinaryNotFound("nginx".into());
        assert_eq!(e.to_string(), "binary not found: nginx");

        let e = Error::CertExpired("example.com".into());
        assert_eq!(e.to_string(), "certificate expired: example.com");

        let e = Error::Other("something went wrong".into());
        assert_eq!(e.to_string(), "something went wrong");
    }
}
