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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::MetadataExt;
    use tempfile::TempDir;

    #[test]
    fn atomic_write_creates_file_with_correct_content() {
        let dir = TempDir::new().expect("temp dir creation should succeed");
        let path = dir.path().join("output.txt");

        atomic_write(&path, "hello world").expect("atomic_write should succeed");

        let content = fs::read_to_string(&path).expect("file should be readable");
        assert_eq!(content, "hello world");
    }

    #[test]
    fn atomic_write_is_atomic_old_or_new_content() {
        let dir = TempDir::new().expect("temp dir creation should succeed");
        let path = dir.path().join("atomic.txt");

        // Write initial content
        fs::write(&path, "old content").expect("initial write should succeed");

        // Overwrite atomically
        atomic_write(&path, "new content").expect("atomic_write should succeed");

        // After the write, the file must have exactly the new content
        let content = fs::read_to_string(&path).expect("file should be readable");
        assert_eq!(content, "new content");
    }

    #[test]
    fn atomic_write_overwrites_existing_file() {
        let dir = TempDir::new().expect("temp dir creation should succeed");
        let path = dir.path().join("overwrite.txt");

        atomic_write(&path, "first").expect("first write should succeed");
        atomic_write(&path, "second").expect("second write should succeed");

        let content = fs::read_to_string(&path).expect("file should be readable");
        assert_eq!(content, "second");
    }

    #[test]
    fn atomic_write_with_perms_sets_0o600() {
        let dir = TempDir::new().expect("temp dir creation should succeed");
        let path = dir.path().join("secret.txt");

        atomic_write_with_perms(&path, "secret data", 0o600)
            .expect("atomic_write_with_perms should succeed");

        let content = fs::read_to_string(&path).expect("file should be readable");
        assert_eq!(content, "secret data");

        let mode = fs::metadata(&path)
            .expect("metadata should be available")
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn atomic_write_with_perms_sets_0o644() {
        let dir = TempDir::new().expect("temp dir creation should succeed");
        let path = dir.path().join("public.txt");

        atomic_write_with_perms(&path, "public data", 0o644)
            .expect("atomic_write_with_perms should succeed");

        let content = fs::read_to_string(&path).expect("file should be readable");
        assert_eq!(content, "public data");

        let mode = fs::metadata(&path)
            .expect("metadata should be available")
            .mode()
            & 0o777;
        assert_eq!(mode, 0o644);
    }

    #[test]
    fn atomic_write_bytes_handles_binary_content() {
        let dir = TempDir::new().expect("temp dir creation should succeed");
        let path = dir.path().join("binary.dat");

        let binary_content: &[u8] = &[0x00, 0xFF, 0x80, 0x7F, 0xDE, 0xAD, 0xBE, 0xEF];
        atomic_write_bytes(&path, binary_content).expect("atomic_write_bytes should succeed");

        let read_back = fs::read(&path).expect("file should be readable");
        assert_eq!(read_back, binary_content);
    }

    #[test]
    fn atomic_write_bytes_handles_empty_content() {
        let dir = TempDir::new().expect("temp dir creation should succeed");
        let path = dir.path().join("empty.dat");

        atomic_write_bytes(&path, &[]).expect("atomic_write_bytes with empty content should succeed");

        let read_back = fs::read(&path).expect("file should be readable");
        assert!(read_back.is_empty());
    }

    #[test]
    fn atomic_write_fails_with_no_parent_directory() {
        let path = Path::new("no_such_dir/output.txt");
        let result = atomic_write(path, "content");
        assert!(result.is_err());
    }
}
