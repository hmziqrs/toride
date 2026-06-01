//! Error types for `toride-diagnostic-types`.

/// Errors returned by diagnostic operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An I/O error occurred (file not found, permission denied, etc.).
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// A required binary was not found on `$PATH`.
    #[error("binary not found: {0}")]
    BinaryNotFound(String),

    /// A render operation failed.
    #[error("render failed: {0}")]
    Render(String),

    /// An uncategorised error.
    #[error("{0}")]
    Other(String),
}

/// Alias for results in this crate.
pub type Result<T> = std::result::Result<T, Error>;
