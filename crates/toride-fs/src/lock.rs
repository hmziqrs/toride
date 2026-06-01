//! Advisory file locking for concurrent write coordination.
//!
//! Uses `fd-lock` to acquire exclusive advisory locks on files. This
//! prevents multiple processes from writing to the same config file
//! simultaneously.
//!
//! # Design
//!
//! The lock is held for the duration of a closure execution (RAII via
//! [`fd_lock::RwLockWriteGuard`]). This avoids self-referential structs
//! and keeps the crate `#![deny(unsafe_code)]` clean.

use std::fs::File;
use std::path::Path;

use tracing;

use crate::error::{Error, Result};

/// Acquire an exclusive advisory lock on `path` and run the given closure
/// while the lock is held.
///
/// Creates the file if it does not exist, acquires an exclusive (write)
/// lock, calls `f`, then drops the lock.
///
/// # Errors
///
/// Returns [`Error::LockFailed`] if the file cannot be opened or the
/// lock cannot be acquired. Returns any error produced by `f`.
pub fn with_lock<F, T>(path: &Path, f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| {
            Error::LockFailed(format!("cannot open lock file {}: {e}", path.display()))
        })?;

    let mut lock = fd_lock::RwLock::new(file);
    let _guard = lock.write().map_err(|e| {
        Error::LockFailed(format!("cannot acquire lock on {}: {e}", path.display()))
    })?;

    tracing::debug!(path = %path.display(), "file lock acquired");
    let result = f();
    tracing::debug!(path = %path.display(), "file lock releasing");
    result
}

/// Acquire an exclusive advisory lock on `path` and run the given closure
/// with the lock file path, while the lock is held.
///
/// This is a convenience variant of [`with_lock`] that passes the lock
/// file path to the closure for informational purposes.
///
/// # Errors
///
/// Returns [`Error::LockFailed`] if the lock cannot be acquired.
/// Returns any error produced by `f`.
pub fn with_lock_path<F, T>(path: &Path, f: F) -> Result<T>
where
    F: FnOnce(&Path) -> Result<T>,
{
    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| {
            Error::LockFailed(format!("cannot open lock file {}: {e}", path.display()))
        })?;

    let mut lock = fd_lock::RwLock::new(file);
    let _guard = lock.write().map_err(|e| {
        Error::LockFailed(format!("cannot acquire lock on {}: {e}", path.display()))
    })?;

    tracing::debug!(path = %path.display(), "file lock acquired");
    let result = f(path);
    tracing::debug!(path = %path.display(), "file lock releasing");
    result
}
