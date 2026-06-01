//! Optional file read utilities.
//!
//! Provides functions that read a file's contents but return `Ok(None)`
//! instead of an error when the file does not exist. Useful for optional
//! configuration files and data files.

use std::fs;
use std::path::Path;

use tracing;

use crate::error::{Error, Result};

/// Read a file's contents as a UTF-8 string, returning `Ok(None)` if the
/// file does not exist.
///
/// # Errors
///
/// Returns [`Error::Io`] if the file exists but cannot be read, or if the
/// contents are not valid UTF-8.
pub fn read_optional(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(content) => {
            tracing::trace!(path = %path.display(), "file read successfully");
            Ok(Some(content))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::trace!(path = %path.display(), "file not found, returning None");
            Ok(None)
        }
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "failed to read file");
            Err(Error::Io(e))
        }
    }
}

/// Read a file's contents as raw bytes, returning `Ok(None)` if the file
/// does not exist.
///
/// # Errors
///
/// Returns [`Error::Io`] if the file exists but cannot be read.
pub fn read_optional_bytes(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(content) => {
            tracing::trace!(path = %path.display(), "file read as bytes successfully");
            Ok(Some(content))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::trace!(path = %path.display(), "file not found, returning None");
            Ok(None)
        }
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "failed to read file");
            Err(Error::Io(e))
        }
    }
}
