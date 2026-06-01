//! Path expansion utilities.
//!
//! Handles tilde (`~`) and environment variable expansion for file paths.
//! Used when resolving user-provided path strings like `~/config` or
//! `~root/.ssh`.

use std::path::PathBuf;

use dirs;

/// Expand a leading `~` in a path string to the current user's home directory.
///
/// Handles:
/// - `~` or `~/...` -- expands to the current user's home directory.
/// - `~user` -- on most Unix systems, this resolves via the `dirs` crate or
///   falls back to `/home/user`. Note: full `~user` resolution requires
///   reading `/etc/passwd` which is not implemented here; only `~` (current
///   user) is fully supported.
/// - No tilde -- returns the path unchanged.
///
/// # Examples
///
/// ```ignore
/// use toride_fs::expand_tilde;
///
/// assert!(expand_tilde("~").starts_with("/home/"));
/// assert_eq!(expand_tilde("/etc/config"), std::path::PathBuf::from("/etc/config"));
/// ```
pub fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        return home_dir();
    }

    if let Some(rest) = path.strip_prefix("~/") {
        return home_dir().join(rest);
    }

    // Handle ~user (basic support -- just use home_dir for current user).
    // Full ~user expansion would require reading /etc/passwd.
    if path.starts_with('~') {
        // Return as-is for ~user patterns we cannot resolve.
        return PathBuf::from(path);
    }

    PathBuf::from(path)
}

/// Expand a path string with tilde and `$HOME` variable support.
///
/// First performs tilde expansion via [`expand_tilde`], then replaces any
/// `$HOME` occurrences with the actual home directory.
///
/// # Examples
///
/// ```ignore
/// use toride_fs::expand_path;
///
/// let resolved = expand_path("$HOME/.config/toride");
/// assert!(resolved.to_string_lossy().contains("/home/"));
/// ```
pub fn expand_path(path: &str) -> PathBuf {
    let expanded = expand_tilde(path);

    // Handle $HOME environment variable.
    let expanded_str = expanded.to_string_lossy();
    if expanded_str.contains("$HOME") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(expanded_str.replace("$HOME", &home.to_string_lossy()));
        }
    }

    expanded
}

/// Returns the current user's home directory.
///
/// Uses the `dirs` crate which respects platform conventions. Falls back
/// to `/root` on Unix if the home directory cannot be determined.
fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/root"))
}
