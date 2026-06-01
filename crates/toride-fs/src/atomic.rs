//! Atomic file write utilities.
//!
//! Provides functions that write data to a temporary file first and then
//! atomically rename it to the target path. This ensures readers never
//! observe a partially-written file.

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use tempfile::NamedTempFile;
use tracing;

use crate::error::{Error, Result};

/// Write a UTF-8 string to `path` atomically.
///
/// Creates a named temporary file in the same directory as `path`, writes
/// the content, and then renames the temp file to `path`. If the rename
/// fails the temp file is cleaned up automatically.
///
/// # Errors
///
/// Returns [`Error::AtomicWriteFailed`] if the temp file cannot be created,
/// written to, or persisted (renamed) to the target path.
pub fn atomic_write(path: &Path, content: &str) -> Result<()> {
    atomic_write_bytes(path, content.as_bytes())
}

/// Write raw bytes to `path` atomically.
///
/// Creates a named temporary file in the same directory as `path`, writes
/// the bytes, and then renames the temp file to `path`.
///
/// # Errors
///
/// Returns [`Error::AtomicWriteFailed`] if the temp file cannot be created,
/// written to, or persisted (renamed) to the target path.
pub fn atomic_write_bytes(path: &Path, content: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or_else(|| Error::AtomicWriteFailed {
        path: path.display().to_string(),
        reason: "path has no parent directory".to_owned(),
    })?;

    let mut tmp = NamedTempFile::new_in(parent).map_err(|e| Error::AtomicWriteFailed {
        path: path.display().to_string(),
        reason: format!("failed to create temp file: {e}"),
    })?;

    tmp.write_all(content).map_err(|e| Error::AtomicWriteFailed {
        path: path.display().to_string(),
        reason: format!("failed to write temp file: {e}"),
    })?;

    tmp.flush().map_err(|e| Error::AtomicWriteFailed {
        path: path.display().to_string(),
        reason: format!("failed to flush temp file: {e}"),
    })?;

    tmp.persist(path).map_err(|e| Error::AtomicWriteFailed {
        path: path.display().to_string(),
        reason: format!("failed to persist temp file: {e}"),
    })?;

    tracing::debug!(path = %path.display(), "atomic write complete");
    Ok(())
}

/// Write a UTF-8 string to `path` atomically, then set file permissions.
///
/// Combines [`atomic_write`] with a `chmod` to the specified `perms` mode
/// (e.g. `0o600` for owner-only read/write).
///
/// # Errors
///
/// Returns [`Error::AtomicWriteFailed`] if the write or persist fails.
/// Returns [`Error::Io`] if the permission change fails.
pub fn atomic_write_with_perms(path: &Path, content: &str, perms: u32) -> Result<()> {
    atomic_write(path, content)?;
    fs::set_permissions(path, fs::Permissions::from_mode(perms))?;
    tracing::debug!(
        path = %path.display(),
        mode = format!("{perms:o}"),
        "atomic write with permissions complete"
    );
    Ok(())
}
