//! File permission helpers.
//!
//! Utilities for setting and checking Unix file permissions and ownership.
//! Used for security-sensitive files like SSH configs, Fail2Ban configs,
//! and other files that must not be world-readable or world-writable.

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use tracing;

use crate::error::{Error, Result};

/// Set file permissions to the given Unix mode.
///
/// `mode` is a raw Unix permission bits value (e.g. `0o600` for
/// owner-only read/write, `0o644` for owner read/write, group/other read).
///
/// # Errors
///
/// Returns [`Error::Io`] if the metadata call or permission change fails.
pub fn set_permissions(path: &Path, mode: u32) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    tracing::debug!(path = %path.display(), mode = format!("{mode:o}"), "permissions set");
    Ok(())
}

/// Verify that a file is **not** world-writable.
///
/// Checks the "other" write bit (`0o002`). If set, returns an error.
///
/// # Errors
///
/// Returns [`Error::PermissionDenied`] if the file is world-writable.
/// Returns [`Error::Io`] if the file metadata cannot be read.
pub fn check_not_world_writable(path: &Path) -> Result<()> {
    let mode = fs::metadata(path)?.mode();
    if mode & 0o002 != 0 {
        return Err(Error::PermissionDenied(format!(
            "{} is world-writable (mode {:o}); refusing to use",
            path.display(),
            mode & 0o777
        )));
    }
    Ok(())
}

/// Check whether the file at `path` is owned by root (uid 0).
///
/// Returns `Ok(true)` if the owner uid is 0, `Ok(false)` otherwise.
///
/// # Errors
///
/// Returns [`Error::Io`] if the file metadata cannot be read.
pub fn check_owner_is_root(path: &Path) -> Result<bool> {
    let uid = fs::metadata(path)?.uid();
    Ok(uid == 0)
}
