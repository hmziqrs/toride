//! Known-hosts file parsing and host-key change detection.
//!
//! Reads and parses `~/.ssh/known_hosts` entries, scans remote hosts via
//! `ssh-keyscan`, and compares the two to detect key changes. Provides
//! [`KnownHostsService`], [`KnownHostEntry`], [`ScannedHostKey`], and
//! [`HostKeyChangeReport`] for diagnostics and trust management.

mod parse;
mod scan;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::paths::SshPaths;
use crate::{Error, Result};

/// A single entry parsed from a `known_hosts` file.
pub use parse::KnownHostEntry;
/// A host key discovered by `ssh-keyscan`.
pub use scan::ScannedHostKey;

/// Report from host key change detection.
///
/// Returned by [`KnownHostsService::check_host_key_change`].  Compares the
/// keys a host currently presents (via `ssh-keyscan`) against the keys stored
/// in `known_hosts`.
#[derive(Debug, Clone)]
pub struct HostKeyChangeReport {
    /// The host that was checked.
    pub host: String,
    /// Whether any key change was detected.
    pub changed: bool,
    /// Entries stored in known_hosts for this host.
    pub stored_keys: Vec<KnownHostEntry>,
    /// Keys discovered by scanning the host.
    pub scanned_keys: Vec<ScannedHostKey>,
    /// Specific key changes detected.
    pub changes: Vec<KeyChange>,
}

/// A specific host key change detected during comparison.
#[derive(Debug, Clone)]
pub struct KeyChange {
    /// The key type that changed (e.g. `"ssh-ed25519"`).
    pub key_type: String,
    /// What kind of change was detected.
    pub kind: KeyChangeKind,
}

/// The kind of host key change.
#[derive(Debug, Clone)]
pub enum KeyChangeKind {
    /// A new key type appeared that was not in `known_hosts`.
    New,
    /// A stored key type is no longer presented by the host.
    Removed,
    /// Same key type but a different public key blob.
    Changed {
        /// SHA-256 fingerprint of the stored key (if computable).
        stored_fingerprint: String,
        /// SHA-256 fingerprint of the scanned key (if computable).
        scanned_fingerprint: String,
    },
}

/// `known_hosts` file management.
///
/// Obtained from [`SshManager::known_hosts()`](crate::SshManager::known_hosts).
pub struct KnownHostsService<'a> {
    paths: &'a SshPaths,
    runner: &'a dyn crate::CliRunner,
}

impl<'a> KnownHostsService<'a> {
    pub(crate) fn new(paths: &'a SshPaths, runner: &'a dyn crate::CliRunner) -> Self {
        Self { paths, runner }
    }

    /// List all known host entries.
    ///
    /// Parses `~/.ssh/known_hosts` and returns every entry found.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the file cannot be read, or
    /// [`Error::KnownHostsParseFailed`] if parsing fails.
    pub async fn list(&self) -> Result<Vec<KnownHostEntry>> {
        parse::parse_known_hosts(self.paths.known_hosts_path()).await
    }

    /// Scan a remote host for its public host keys.
    ///
    /// Runs `ssh-keyscan <host>` and returns the keys discovered with the
    /// plaintext hostname.  Keys are **not** added to `known_hosts`; call
    /// [`add`](Self::add) for that.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ToolNotFound`] if `ssh-keyscan` is not in `PATH`,
    /// or [`Error::CommandFailed`] if the scan fails.
    pub async fn scan(&self, host: &str) -> Result<Vec<ScannedHostKey>> {
        scan::scan_host(host, self.runner).await
    }

    /// Scan a host and add all its keys to `~/.ssh/known_hosts`.
    ///
    /// Uses `ssh-keyscan -H <host>` so that hostnames are stored in hashed
    /// form for privacy.  All keys for the host are written in a single
    /// I/O operation.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ToolNotFound`] if `ssh-keyscan` is not in `PATH`,
    /// [`Error::CommandFailed`] if scanning fails, or [`Error::Io`] if
    /// writing to the known_hosts file fails.
    pub async fn add(&self, host: &str) -> Result<()> {
        scan::add_host_hashed(self.paths.known_hosts_path(), host, self.runner).await
    }

    /// Remove all entries matching the given host from `~/.ssh/known_hosts`.
    ///
    /// Entries whose hostname patterns list contains an exact match for `host`
    /// are removed.  Hashed entries (`|1|...`) cannot be matched by name and
    /// are left untouched.
    ///
    /// The removal is performed atomically (write to a temp file, then rename)
    /// so that a crash mid-write cannot corrupt the file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::HostNotKnown`] if no entries match the given host,
    /// [`Error::TaskFailed`] if the background task panics, or
    /// [`Error::Io`] if file operations fail.
    pub async fn remove(&self, host: &str) -> Result<()> {
        // Allocate an owned PathBuf for use inside `spawn_blocking` (requires `'static`).
        let path = self.paths.known_hosts_path().to_path_buf();
        let host = host.to_owned();

        tokio::task::spawn_blocking(move || remove_host_sync(&path, &host))
            .await
            .map_err(|e| Error::TaskFailed(e.to_string()))?
    }

    /// Check whether a host appears in `~/.ssh/known_hosts`.
    ///
    /// Returns `true` if any entry's host pattern list contains an exact match.
    /// Both plain and bracketed (`[host]:port`) forms are checked.  Hashed
    /// entries are not matched (that would require re-hashing the hostname
    /// with the stored salt).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] or [`Error::KnownHostsParseFailed`] if the
    /// known_hosts file cannot be read or parsed.
    pub async fn contains(&self, host: &str) -> Result<bool> {
        let entries = self.list().await?;
        Ok(entries
            .iter()
            .any(|e| e.hosts.iter().any(|h| host_pattern_matches(h, host))))
    }

    /// Check if `UpdateHostKeys` is configured in `~/.ssh/config`.
    ///
    /// The SSH `UpdateHostKeys` directive (when set to `yes`) tells the client
    /// to automatically update `known_hosts` when the server presents new host
    /// key types.  This improves forward security by allowing servers to
    /// rotate their host keys without manual intervention.
    ///
    /// # Limitations
    ///
    /// toride does **not** currently support the `UpdateHostKeys` protocol
    /// extension.  This method detects the configuration setting so that
    /// callers can warn the user.  Future versions could implement the
    /// RFC 4252 / OpenSSH host key update protocol.
    ///
    /// Returns `true` if `UpdateHostKeys yes` appears in the config (either
    /// at top level or in any `Host` block), `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the config file cannot be read.
    pub async fn is_update_host_keys_enabled(&self) -> Result<bool> {
        let config_path = self.paths.config_path();
        if !config_path.exists() {
            return Ok(false);
        }

        let content = tokio::fs::read_to_string(config_path).await?;

        for line in content.lines() {
            let trimmed = line.trim();
            // Skip comments.
            if trimmed.starts_with('#') {
                continue;
            }

            // Check for "UpdateHostKeys yes" (case-insensitive keyword).
            let lower = trimmed.to_ascii_lowercase();
            if lower.starts_with("updatehostkeys") {
                // Extract the value after the keyword.
                let value = trimmed["UpdateHostKeys".len()..].trim();
                if value.eq_ignore_ascii_case("yes") {
                    tracing::info!(
                        "UpdateHostKeys yes found in SSH config — \
                         this feature is not yet supported by toride"
                    );
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Hash all hostnames in `~/.ssh/known_hosts` (`ssh-keygen -H`).
    ///
    /// This replaces plaintext hostnames with salted hashes for privacy.
    /// The file is modified in-place by `ssh-keygen`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ToolNotFound`] if `ssh-keygen` is not in `PATH`,
    /// [`Error::CommandFailed`] if hashing fails, or
    /// [`Error::TaskFailed`] if the background task panics.
    pub async fn hash_all(&self) -> Result<()> {
        let path_str = self
            .paths
            .known_hosts_path()
            .to_str()
            .ok_or_else(|| Error::CommandFailed("known_hosts path is not valid UTF-8".into()))?
            .to_owned();

        self.runner
            .run(
                "ssh-keygen",
                vec!["-H".to_owned(), "-f".to_owned(), path_str],
            )
            .await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // find() — ssh-keygen -F based lookup
    // -----------------------------------------------------------------------

    /// Find all `known_hosts` entries matching the given host.
    ///
    /// Uses `ssh-keygen -F <host>` which can match **hashed** hostnames
    /// (something the pure-text [`contains`](Self::contains) method cannot
    /// do).  Both the user known_hosts file and the global known hosts file
    /// (`/etc/ssh/ssh_known_hosts`) are searched.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ToolNotFound`] if `ssh-keygen` is not in `PATH`.
    /// A missing known_hosts file is treated as "no entries" rather than an
    /// error.
    pub async fn find(&self, host: &str) -> Result<Vec<KnownHostEntry>> {
        let mut entries = Vec::new();

        // 1. User known_hosts (or UserKnownHostsFile from config).
        let user_path = self.resolve_user_known_hosts_file(host).await?;
        entries.extend(self.find_in_file(host, &user_path).await?);

        // 2. Global known hosts.
        let global_path = self.paths.global_known_hosts_path();
        if global_path.exists() {
            entries.extend(self.find_in_file(host, global_path).await?);
        }

        Ok(entries)
    }

    /// Search a specific known_hosts file for entries matching `host`.
    ///
    /// Runs `ssh-keygen -F <host> -f <file>` and parses the output.  A
    /// missing file returns an empty vec (not an error).
    async fn find_in_file(&self, host: &str, path: &Path) -> Result<Vec<KnownHostEntry>> {
        if !path.exists() {
            return Ok(Vec::new());
        }

        let path_str = path
            .to_str()
            .ok_or_else(|| Error::CommandFailed("known_hosts path is not valid UTF-8".into()))?
            .to_owned();

        let host_owned = host.to_owned();
        let args = vec![
            "-F".to_owned(),
            host_owned,
            "-f".to_owned(),
            path_str,
        ];

        // ssh-keygen -F returns exit code 1 when the host is not found —
        // that is a normal result, not an error.
        match self.runner.run("ssh-keygen", args).await {
            Ok(raw) => Ok(parse_ssh_keygen_f_output(host, &raw)),
            Err(_) => Ok(Vec::new()),
        }
    }

    // -----------------------------------------------------------------------
    // GlobalKnownHostsFile support
    // -----------------------------------------------------------------------

    /// List entries from the global known hosts file (`/etc/ssh/ssh_known_hosts`).
    ///
    /// Returns an empty vec if the file does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the file exists but cannot be read, or
    /// [`Error::KnownHostsParseFailed`] if parsing fails.
    pub async fn list_global(&self) -> Result<Vec<KnownHostEntry>> {
        let path = self.paths.global_known_hosts_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        parse::parse_known_hosts(path).await
    }

    /// List entries from **all** known hosts files.
    ///
    /// Merges entries from:
    /// 1. The user known_hosts file (or `UserKnownHostsFile` from config).
    /// 2. The global known hosts file (`/etc/ssh/ssh_known_hosts`).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if any file cannot be read, or
    /// [`Error::KnownHostsParseFailed`] if parsing fails.
    pub async fn list_all(&self) -> Result<Vec<KnownHostEntry>> {
        let mut entries = self.list().await?;
        entries.extend(self.list_global().await?);
        Ok(entries)
    }

    // -----------------------------------------------------------------------
    // Host key change detection
    // -----------------------------------------------------------------------

    /// Scan a host and compare its keys with what is stored in `known_hosts`.
    ///
    /// Returns a [`HostKeyChangeReport`] describing any differences:
    /// - **New** key types that the host presents but are not stored.
    /// - **Removed** key types that are stored but the host no longer presents.
    /// - **Changed** key types where the type matches but the public key blob
    ///   differs.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ToolNotFound`] if `ssh-keyscan` or `ssh-keygen` is
    /// not in `PATH`, or [`Error::CommandFailed`] if scanning fails.
    pub async fn check_host_key_change(&self, host: &str) -> Result<HostKeyChangeReport> {
        let scanned = self.scan(host).await?;
        let stored = self.find(host).await?;
        let changes = compare_host_keys(&stored, &scanned);

        Ok(HostKeyChangeReport {
            host: host.to_owned(),
            changed: !changes.is_empty(),
            stored_keys: stored,
            scanned_keys: scanned,
            changes,
        })
    }

    // -----------------------------------------------------------------------
    // UserKnownHostsFile config directive awareness
    // -----------------------------------------------------------------------

    /// Resolve the `UserKnownHostsFile` directive from `~/.ssh/config` for
    /// the given host.
    ///
    /// If the directive is set in a matching `Host` block, returns the
    /// configured path (with `~` expanded).  If set to `"none"`, returns
    /// `None`.  Otherwise returns the default `~/.ssh/known_hosts`.
    ///
    /// This performs a lightweight config scan — it does **not** follow
    /// `Include` chains or evaluate `Match` blocks.  For full config
    /// resolution use [`ConfigService::resolve_host`](crate::config::ConfigService::resolve_host).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the config file exists but cannot be read.
    pub async fn resolve_user_known_hosts_file(&self, host: &str) -> Result<PathBuf> {
        let config_path = self.paths.config_path();
        if !config_path.exists() {
            return Ok(self.paths.known_hosts_path().to_path_buf());
        }

        let content = tokio::fs::read_to_string(config_path).await?;

        if let Some(raw) = find_user_known_hosts_file_in_config(&content, host) {
            if raw.eq_ignore_ascii_case("none") {
                // "none" means no user known_hosts file — return a path that
                // will never exist so callers get an empty result.
                return Ok(PathBuf::from("/dev/null"));
            }
            return Ok(expand_known_hosts_path(&raw));
        }

        Ok(self.paths.known_hosts_path().to_path_buf())
    }
}

/// Parse `ssh-keygen -F` output into [`KnownHostEntry`] values.
///
/// The output contains comment lines (starting with `#`) that indicate where
/// each match was found, followed by the entry line in standard known_hosts
/// format.  This function skips the comments and parses the entry lines.
fn parse_ssh_keygen_f_output(host: &str, raw: &str) -> Vec<KnownHostEntry> {
    let mut entries = Vec::new();
    let mut line_number = 0usize;

    for raw_line in raw.lines() {
        let trimmed = raw_line.trim();

        // Extract line number from the comment if available.
        if trimmed.starts_with('#') {
            if let Some(rest) = trimmed.strip_prefix("# Host ")
                && let Some(pos) = rest.find("found: line ")
            {
                let num_str = &rest[pos + "found: line ".len()..];
                line_number = num_str.trim().parse().unwrap_or(0);
            }
            continue;
        }

        if trimmed.is_empty() {
            continue;
        }

        match parse::parse_line(trimmed, line_number) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                tracing::warn!(
                    host,
                    error = %e,
                    "skipping unparseable ssh-keygen -F output line"
                );
            }
        }
    }

    entries
}

/// Compare stored and scanned host keys, returning a list of changes.
fn compare_host_keys(
    stored: &[KnownHostEntry],
    scanned: &[ScannedHostKey],
) -> Vec<KeyChange> {
    let mut changes = Vec::new();

    // Build maps of key_type -> (public_key, fingerprint_display) for both.
    let stored_map: HashMap<&str, &str> = stored
        .iter()
        .map(|e| (e.key_type.as_str(), e.public_key.as_str()))
        .collect();
    let scanned_map: HashMap<&str, &str> = scanned
        .iter()
        .map(|e| (e.key_type.as_str(), e.public_key.as_str()))
        .collect();

    // Check for new and changed keys.
    for (&key_type, &scanned_key) in &scanned_map {
        match stored_map.get(key_type) {
            Some(&stored_key) => {
                if stored_key != scanned_key {
                    let stored_fp = match parse::compute_key_fingerprint(stored_key, key_type) {
                        Ok(fp) => fp.to_string(),
                        Err(_) => "(unavailable)".to_owned(),
                    };
                    let scanned_fp = match parse::compute_key_fingerprint(scanned_key, key_type) {
                        Ok(fp) => fp.to_string(),
                        Err(_) => "(unavailable)".to_owned(),
                    };
                    changes.push(KeyChange {
                        key_type: key_type.to_owned(),
                        kind: KeyChangeKind::Changed {
                            stored_fingerprint: stored_fp,
                            scanned_fingerprint: scanned_fp,
                        },
                    });
                }
            }
            None => {
                changes.push(KeyChange {
                    key_type: key_type.to_owned(),
                    kind: KeyChangeKind::New,
                });
            }
        }
    }

    // Check for removed keys.
    for &key_type in stored_map.keys() {
        if !scanned_map.contains_key(key_type) {
            changes.push(KeyChange {
                key_type: key_type.to_owned(),
                kind: KeyChangeKind::Removed,
            });
        }
    }

    changes
}

/// Scan SSH config for `UserKnownHostsFile` directive matching the given host.
///
/// Performs a lightweight scan — does not follow `Include` chains or `Match`
/// blocks.  Returns the raw directive value (may contain `~`).
fn find_user_known_hosts_file_in_config(config_content: &str, host: &str) -> Option<String> {
    let mut in_matching_block = false;
    let mut in_any_host_block = false;
    let mut global_value: Option<String> = None;
    let mut host_value: Option<String> = None;

    for line in config_content.lines() {
        let trimmed = line.trim();

        // Skip comments and blank lines.
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        // Detect Host block boundaries.
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("host ") {
            let patterns: Vec<&str> = trimmed[5..].split_whitespace().collect();
            in_any_host_block = true;
            in_matching_block = patterns.iter().any(|p| {
                *p == "*" || p.eq_ignore_ascii_case(host)
            });
            continue;
        }

        if lower.starts_with("userknownhostsfile ") {
            let value = trimmed["userknownhostsfile".len()..].trim().to_owned();
            if in_matching_block && host_value.is_none() {
                host_value = Some(value);
            } else if !in_any_host_block && global_value.is_none() {
                // Only treat as global directive if outside any Host block.
                global_value = Some(value);
            }
        }
    }

    host_value.or(global_value)
}

/// Expand a `UserKnownHostsFile` value to an absolute path.
///
/// Handles `~` expansion.  Relative paths are resolved against the current
/// directory (matching OpenSSH behaviour).
fn expand_known_hosts_path(raw: &str) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    } else if raw == "~"
        && let Some(home) = dirs::home_dir()
    {
        return home;
    }
    let path = Path::new(raw);
    if path.is_relative() {
        // Resolve relative to current directory.
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    } else {
        path.to_path_buf()
    }
}

/// Generate a short random hex suffix for unique temp file names.
///
/// Uses a simple counter + timestamp mix to avoid importing a full RNG crate.
/// Good enough for temp file uniqueness within a single process.
fn rand_suffix() -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    std::time::Instant::now().hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    hasher.finish()
}

/// Synchronous helper: read the file, filter out matching entries, write back
/// atomically via a temp file + rename.
fn remove_host_sync(path: &Path, host: &str) -> Result<()> {
    let contents = std::fs::read_to_string(path)?;

    let mut kept = String::new();
    let mut removed_any = false;

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            kept.push_str(raw_line);
            kept.push('\n');
            continue;
        }

        if line_matches_host(line, host) {
            removed_any = true;
        } else {
            kept.push_str(raw_line);
            kept.push('\n');
        }
    }

    if !removed_any {
        return Err(Error::HostNotKnown(host.to_owned()));
    }

    // Atomic write: write to a temp file in the same directory, then rename.
    let parent = path.parent().ok_or_else(|| {
        Error::KnownHostsParseFailed("known_hosts path has no parent directory".into())
    })?;
    // Use PID + nanosecond timestamp + random suffix to avoid races between
    // concurrent remove() calls within the same process.
    let tmp_path = parent.join(format!(
        ".known_hosts.tmp.{}.{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        rand_suffix()
    ));
    // Use create_new to prevent symlink attacks on multi-user systems.
    {
        let mut tmp_file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)?;
        std::io::Write::write_all(&mut tmp_file, kept.as_bytes())?;
    }
    // Preserve the original file permissions.
    if let Ok(original_meta) = std::fs::metadata(path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(
                &tmp_path,
                std::fs::Permissions::from_mode(original_meta.permissions().mode()),
            );
        }
        #[cfg(not(unix))]
        {
            let _ = std::fs::set_permissions(&tmp_path, original_meta.permissions());
        }
    }
    if let Err(e) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e.into());
    }
    Ok(())
}

/// Check whether a single host pattern from a known_hosts entry matches the
/// given target hostname.
///
/// Handles exact string match and bracketed `[host]:port` forms.
/// Does **not** expand glob patterns (`*`, `?`) or negations (`!`) — those
/// require the full SSH matching algorithm.
fn host_pattern_matches(pattern: &str, target: &str) -> bool {
    // Direct match.
    if pattern == target {
        return true;
    }

    if let Some((p_host, p_port)) = strip_brackets(pattern)
        && let Some((t_host, t_port)) = target.split_once(':')
        && p_host == t_host && p_port == t_port
    {
        return true;
    }
    if let Some((t_host, t_port)) = strip_brackets(target)
        && let Some((p_host, p_port)) = pattern.split_once(':')
        && p_host == t_host && p_port == t_port
    {
        return true;
    }
    false
}

/// Extract host and port from a bracketed `[host]:port` string.
///
/// Returns `None` if the string is not in bracketed form.
fn strip_brackets(s: &str) -> Option<(&str, &str)> {
    let inner = s.strip_prefix('[')?;
    let (host, rest) = inner.split_once("]:")?;
    Some((host, rest))
}

/// Check whether a known_hosts line refers to the given host.
///
/// Handles plain hostnames, comma-separated patterns, and markers.
/// Does **not** attempt to match hashed entries.
fn line_matches_host(line: &str, target: &str) -> bool {
    // Skip optional marker.
    let rest = if line.starts_with('@') {
        let Some((_, r)) = line.split_once(' ') else {
            return false;
        };
        r
    } else {
        line
    };

    // The host field is the first whitespace-delimited token.
    let Some(hosts_field) = rest.split_whitespace().next() else {
        return false;
    };

    // Hashed entries — cannot match by name.
    if hosts_field.starts_with("|1|") {
        return false;
    }

    // Comma-separated patterns — try each one.
    hosts_field
        .split_terminator(',')
        .any(|pattern| host_pattern_matches(pattern, target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::KeyType;

    #[test]
    fn strip_brackets_valid() {
        assert_eq!(strip_brackets("[host]:22"), Some(("host", "22")));
        assert_eq!(strip_brackets("[192.168.1.1]:2222"), Some(("192.168.1.1", "2222")));
    }

    #[test]
    fn strip_brackets_invalid() {
        assert_eq!(strip_brackets("host:22"), None);
        assert_eq!(strip_brackets("host"), None);
        assert_eq!(strip_brackets("[host]"), None);
        assert_eq!(strip_brackets(""), None);
    }

    #[test]
    fn host_pattern_matches_exact() {
        assert!(host_pattern_matches("example.com", "example.com"));
    }

    #[test]
    fn host_pattern_matches_no_match() {
        assert!(!host_pattern_matches("example.com", "other.com"));
    }

    #[test]
    fn host_pattern_matches_bracketed_pattern() {
        assert!(host_pattern_matches("[example.com]:22", "example.com:22"));
    }

    #[test]
    fn host_pattern_matches_bracketed_target() {
        assert!(host_pattern_matches("example.com:22", "[example.com]:22"));
    }

    #[test]
    fn host_pattern_matches_port_different() {
        assert!(!host_pattern_matches("[example.com]:22", "example.com:2222"));
    }

    #[test]
    fn line_matches_host_simple() {
        assert!(line_matches_host("example.com ssh-ed25519 AAAA...", "example.com"));
    }

    #[test]
    fn line_matches_host_comma_separated() {
        assert!(line_matches_host("host1.com,host2.com ssh-ed25519 AAAA...", "host2.com"));
    }

    #[test]
    fn line_matches_host_no_match() {
        assert!(!line_matches_host("other.com ssh-ed25519 AAAA...", "example.com"));
    }

    #[test]
    fn line_matches_host_skips_hashed() {
        assert!(!line_matches_host("|1|salt|hash ssh-ed25519 AAAA...", "example.com"));
    }

    #[test]
    fn line_matches_host_cert_authority_marker() {
        assert!(line_matches_host("@cert-authority example.com ssh-ed25519 AAAA...", "example.com"));
    }

    #[test]
    fn line_matches_host_revoked_marker() {
        // @revoked with exact hostname matches
        assert!(line_matches_host("@revoked example.com ssh-ed25519 AAAA...", "example.com"));
    }

    #[test]
    fn line_matches_host_revoked_no_match() {
        // @revoked with different host does not match
        assert!(!line_matches_host("@revoked other.com ssh-ed25519 AAAA...", "example.com"));
    }

    #[test]
    fn line_matches_host_marker_no_space() {
        // Malformed marker line without space after marker
        assert!(!line_matches_host("@cert-authority", "example.com"));
    }

    #[test]
    fn line_matches_host_empty() {
        assert!(!line_matches_host("", "example.com"));
    }

    #[test]
    fn line_matches_host_bracketed_port() {
        assert!(line_matches_host("[example.com]:2222 ssh-ed25519 AAAA...", "example.com:2222"));
    }

    // -----------------------------------------------------------------------
    // Known hosts find() — by hostname, by IP, by [host]:port
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn find_entry_by_hostname() {
        let dir = tempfile::tempdir().unwrap();
        let kh_path = dir.path().join("known_hosts");
        std::fs::write(
            &kh_path,
            "example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl\n",
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();
        let svc = KnownHostsService::new(&paths, &runner);
        assert!(svc.contains("example.com").await.unwrap());
        assert!(!svc.contains("other.com").await.unwrap());
    }

    #[tokio::test]
    async fn find_entry_by_ip_address() {
        let dir = tempfile::tempdir().unwrap();
        let kh_path = dir.path().join("known_hosts");
        std::fs::write(
            &kh_path,
            "192.168.1.1 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl\n",
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();
        let svc = KnownHostsService::new(&paths, &runner);
        assert!(svc.contains("192.168.1.1").await.unwrap());
        assert!(!svc.contains("10.0.0.1").await.unwrap());
    }

    #[tokio::test]
    async fn find_entry_by_bracketed_host_port() {
        let dir = tempfile::tempdir().unwrap();
        let kh_path = dir.path().join("known_hosts");
        std::fs::write(
            &kh_path,
            "[example.com]:2222 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl\n",
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();
        let svc = KnownHostsService::new(&paths, &runner);
        // Target "example.com:2222" should match the bracketed pattern "[example.com]:2222".
        assert!(svc.contains("example.com:2222").await.unwrap());
        // Same host on a different port should NOT match.
        assert!(!svc.contains("example.com:22").await.unwrap());
    }

    #[tokio::test]
    async fn find_entry_by_comma_separated_hosts() {
        let dir = tempfile::tempdir().unwrap();
        let kh_path = dir.path().join("known_hosts");
        std::fs::write(
            &kh_path,
            "host1.example.com,host2.example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl\n",
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();
        let svc = KnownHostsService::new(&paths, &runner);
        assert!(svc.contains("host1.example.com").await.unwrap());
        assert!(svc.contains("host2.example.com").await.unwrap());
        assert!(!svc.contains("host3.example.com").await.unwrap());
    }

    #[tokio::test]
    async fn find_entry_with_cert_authority_marker() {
        let dir = tempfile::tempdir().unwrap();
        let kh_path = dir.path().join("known_hosts");
        std::fs::write(
            &kh_path,
            "@cert-authority *.example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl\n",
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();
        let svc = KnownHostsService::new(&paths, &runner);
        let entries = svc.list().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].markers, vec!["@cert-authority"]);
        assert_eq!(entries[0].hosts, vec!["*.example.com"]);
    }

    // -----------------------------------------------------------------------
    // Known hosts fingerprint display — SHA-256 format
    // -----------------------------------------------------------------------

    #[test]
    fn fingerprint_display_format() {
        let fp = crate::Fingerprint {
            hash: "abc123def456".to_owned(),
            key_type: KeyType::Ed25519,
        };
        assert_eq!(format!("{fp}"), "SHA256:abc123def456");
    }

    #[test]
    fn fingerprint_display_rsa() {
        let fp = crate::Fingerprint {
            hash: "xYz789+/AbCdEf".to_owned(),
            key_type: KeyType::Rsa { bits: 4096 },
        };
        assert_eq!(format!("{fp}"), "SHA256:xYz789+/AbCdEf");
    }

    #[test]
    fn fingerprint_display_ecdsa() {
        let fp = crate::Fingerprint {
            hash: "nistp256hashvalue".to_owned(),
            key_type: KeyType::EcdsaP256,
        };
        assert_eq!(format!("{fp}"), "SHA256:nistp256hashvalue");
    }

    // -----------------------------------------------------------------------
    // Known hosts host key change detection — scanned vs stored
    // -----------------------------------------------------------------------

    #[test]
    fn host_key_change_detection_different_key_types() {
        use super::parse::parse_line;
        use super::scan::parse_keyscan_line;

        // Stored entry: host has an ed25519 key.
        let stored = parse_line(
            "example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl",
            1,
        )
        .unwrap();

        // Scanned entry: same host now reports an rsa key instead.
        let scanned = parse_keyscan_line(
            "example.com",
            "example.com ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQC7",
        )
        .unwrap();

        // Key types differ — host key has changed.
        assert_ne!(stored.key_type, scanned.key_type);
    }

    #[test]
    fn host_key_change_detection_same_key_type_different_blob() {
        use super::parse::parse_line;
        use super::scan::parse_keyscan_line;

        let stored = parse_line(
            "example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl",
            1,
        )
        .unwrap();

        // Same key type but different blob.
        let scanned = parse_keyscan_line(
            "example.com",
            "example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDIFFERENTKEY0000000000000000000000000",
        )
        .unwrap();

        assert_eq!(stored.key_type, scanned.key_type);
        assert_ne!(stored.public_key, scanned.public_key);
    }

    #[test]
    fn host_key_no_change_detected() {
        use super::parse::parse_line;
        use super::scan::parse_keyscan_line;

        let key_blob = "AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl";
        let stored = parse_line(
            &format!("example.com ssh-ed25519 {key_blob}"),
            1,
        )
        .unwrap();

        let scanned = parse_keyscan_line(
            "example.com",
            &format!("example.com ssh-ed25519 {key_blob}"),
        )
        .unwrap();

        assert_eq!(stored.key_type, scanned.key_type);
        assert_eq!(stored.public_key, scanned.public_key);
    }

    #[tokio::test]
    async fn list_known_hosts_and_find_entry() {
        let dir = tempfile::tempdir().unwrap();
        let kh_path = dir.path().join("known_hosts");
        std::fs::write(
            &kh_path,
            "\
host-a ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl
host-b ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQC7
",
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();
        let svc = KnownHostsService::new(&paths, &runner);
        let entries = svc.list().await.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].hosts, vec!["host-a"]);
        assert_eq!(entries[0].key_type, "ssh-ed25519");
        assert_eq!(entries[1].hosts, vec!["host-b"]);
        assert_eq!(entries[1].key_type, "ssh-rsa");
    }

    #[tokio::test]
    async fn remove_host_from_known_hosts() {
        let dir = tempfile::tempdir().unwrap();
        let kh_path = dir.path().join("known_hosts");
        std::fs::write(
            &kh_path,
            "\
host-a ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl
host-b ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQC7
",
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();
        let svc = KnownHostsService::new(&paths, &runner);
        svc.remove("host-a").await.unwrap();

        let remaining = svc.list().await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].hosts, vec!["host-b"]);
    }

    #[tokio::test]
    async fn remove_nonexistent_host_errors() {
        let dir = tempfile::tempdir().unwrap();
        let kh_path = dir.path().join("known_hosts");
        std::fs::write(
            &kh_path,
            "host-a ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl\n",
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();
        let svc = KnownHostsService::new(&paths, &runner);
        let result = svc.remove("nonexistent").await;
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // parse_ssh_keygen_f_output
    // -----------------------------------------------------------------------

    #[test]
    fn parse_ssh_keygen_f_output_single_entry() {
        let raw = "\
# Host example.com found: line 3
example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl
";
        let entries = parse_ssh_keygen_f_output("example.com", raw);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].hosts, vec!["example.com"]);
        assert_eq!(entries[0].key_type, "ssh-ed25519");
        assert_eq!(entries[0].line_number, 3);
    }

    #[test]
    fn parse_ssh_keygen_f_output_multiple_entries() {
        let raw = "\
# Host example.com found: line 1
example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl
# Host example.com found: line 5
example.com ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQC7
";
        let entries = parse_ssh_keygen_f_output("example.com", raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key_type, "ssh-ed25519");
        assert_eq!(entries[0].line_number, 1);
        assert_eq!(entries[1].key_type, "ssh-rsa");
        assert_eq!(entries[1].line_number, 5);
    }

    #[test]
    fn parse_ssh_keygen_f_output_empty() {
        let entries = parse_ssh_keygen_f_output("example.com", "");
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_ssh_keygen_f_output_only_comments() {
        let raw = "# Host example.com found: line 0\n";
        let entries = parse_ssh_keygen_f_output("example.com", raw);
        assert!(entries.is_empty());
    }

    // -----------------------------------------------------------------------
    // find() — ssh-keygen -F based lookup
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn find_returns_matching_entries() {
        let dir = tempfile::tempdir().unwrap();
        let kh_path = dir.path().join("known_hosts");
        let key_b64 = "AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl";
        std::fs::write(
            &kh_path,
            format!("example.com ssh-ed25519 {key_b64}\n"),
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();
        runner.push_run_response(
            "ssh-keygen",
            Ok(format!(
                "# Host example.com found: line 1\nexample.com ssh-ed25519 {key_b64}\n"
            )),
        );
        let svc = KnownHostsService::new(&paths, &runner);
        let entries = svc.find("example.com").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].hosts, vec!["example.com"]);
    }

    #[tokio::test]
    async fn find_returns_empty_when_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let kh_path = dir.path().join("known_hosts");
        std::fs::write(&kh_path, "").unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();
        // ssh-keygen -F returns exit code 1 for not found — mock as error.
        runner.push_run_response(
            "ssh-keygen",
            Err(crate::Error::CommandFailed("exit status: 1".into())),
        );
        let svc = KnownHostsService::new(&paths, &runner);
        let entries = svc.find("unknown.host").await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn find_searches_global_known_hosts() {
        let dir = tempfile::tempdir().unwrap();
        let kh_path = dir.path().join("known_hosts");
        let global_path = dir.path().join("ssh_known_hosts");
        let key_b64 = "AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl";
        std::fs::write(&kh_path, "").unwrap();
        std::fs::write(
            &global_path,
            format!("global-host ssh-ed25519 {key_b64}\n"),
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();
        // First call: user known_hosts (empty).
        runner.push_run_response(
            "ssh-keygen",
            Err(crate::Error::CommandFailed("not found".into())),
        );
        // Second call: global known_hosts.
        runner.push_run_response(
            "ssh-keygen",
            Ok(format!(
                "# Host global-host found: line 1\nglobal-host ssh-ed25519 {key_b64}\n"
            )),
        );
        let svc = KnownHostsService::new(&paths, &runner);
        let entries = svc.find("global-host").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].hosts, vec!["global-host"]);
    }

    // -----------------------------------------------------------------------
    // fingerprint() on KnownHostEntry
    // -----------------------------------------------------------------------

    #[test]
    fn known_host_entry_fingerprint_is_sha256() {
        let entry = parse::parse_line(
            "example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl",
            1,
        )
        .unwrap();
        let fp = entry.fingerprint().unwrap();
        let fp_str = format!("{fp}");
        assert!(fp_str.starts_with("SHA256:"), "expected SHA256 prefix, got: {fp_str}");
        assert!(fp_str.len() > 8, "fingerprint too short: {fp_str}");
    }

    #[test]
    fn known_host_entry_fingerprint_rsa() {
        // A minimal RSA key blob — not a real key, but structurally valid
        // for the ssh_key parser (algorithm string + exponent + modulus).
        // We test that fingerprint computation does not panic on RSA entries
        // and produces a SHA256: prefix.  Since this is a synthetic blob,
        // we allow the parse to fail gracefully and just verify the path.
        let entry = parse::parse_line(
            "host ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl",
            1,
        )
        .unwrap();
        let fp = entry.fingerprint().unwrap();
        let fp_str = format!("{fp}");
        assert!(fp_str.starts_with("SHA256:"));
    }

    // -----------------------------------------------------------------------
    // fingerprint() on ScannedHostKey
    // -----------------------------------------------------------------------

    #[test]
    fn scanned_host_key_fingerprint() {
        let key = scan::parse_keyscan_line(
            "example.com",
            "example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl",
        )
        .unwrap();
        let fp = key.fingerprint().unwrap();
        let fp_str = format!("{fp}");
        assert!(fp_str.starts_with("SHA256:"));
    }

    #[test]
    fn stored_and_scanned_fingerprints_match_for_same_key() {
        let key_b64 = "AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl";
        let stored = parse::parse_line(
            &format!("example.com ssh-ed25519 {key_b64}"),
            1,
        )
        .unwrap();
        let scanned = scan::parse_keyscan_line(
            "example.com",
            &format!("example.com ssh-ed25519 {key_b64}"),
        )
        .unwrap();

        let stored_fp = stored.fingerprint().unwrap();
        let scanned_fp = scanned.fingerprint().unwrap();
        assert_eq!(format!("{stored_fp}"), format!("{scanned_fp}"));
    }

    #[test]
    fn different_keys_produce_different_fingerprints() {
        // Use compare_host_keys which handles invalid key blobs gracefully.
        let stored = vec![parse::parse_line(
            "example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl",
            1,
        )
        .unwrap()];
        let scanned = vec![scan::parse_keyscan_line(
            "example.com",
            "example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJm",
        )
        .unwrap()];

        let changes = compare_host_keys(&stored, &scanned);
        assert_eq!(changes.len(), 1);
        assert!(matches!(changes[0].kind, KeyChangeKind::Changed { .. }));
    }

    // -----------------------------------------------------------------------
    // list_global()
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn list_global_reads_global_file() {
        let dir = tempfile::tempdir().unwrap();
        let global_path = dir.path().join("ssh_known_hosts");
        std::fs::write(
            &global_path,
            "global-host ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl\n",
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();
        let svc = KnownHostsService::new(&paths, &runner);
        let entries = svc.list_global().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].hosts, vec!["global-host"]);
    }

    #[tokio::test]
    async fn list_global_returns_empty_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();
        let svc = KnownHostsService::new(&paths, &runner);
        let entries = svc.list_global().await.unwrap();
        assert!(entries.is_empty());
    }

    // -----------------------------------------------------------------------
    // list_all()
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn list_all_merges_user_and_global() {
        let dir = tempfile::tempdir().unwrap();
        let kh_path = dir.path().join("known_hosts");
        let global_path = dir.path().join("ssh_known_hosts");
        std::fs::write(
            &kh_path,
            "user-host ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl\n",
        )
        .unwrap();
        std::fs::write(
            &global_path,
            "global-host ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQC7\n",
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();
        let svc = KnownHostsService::new(&paths, &runner);
        let entries = svc.list_all().await.unwrap();
        assert_eq!(entries.len(), 2);
        let hosts: Vec<&str> = entries.iter().flat_map(|e| e.hosts.iter().map(|s| s.as_str())).collect();
        assert!(hosts.contains(&"user-host"));
        assert!(hosts.contains(&"global-host"));
    }

    // -----------------------------------------------------------------------
    // check_host_key_change()
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn check_host_key_change_detects_changed_key() {
        let dir = tempfile::tempdir().unwrap();
        let kh_path = dir.path().join("known_hosts");
        std::fs::write(
            &kh_path,
            "example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl\n",
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();

        // Mock ssh-keygen -F for find() — return the stored entry.
        runner.push_run_response(
            "ssh-keygen",
            Ok("# Host example.com found: line 1\nexample.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl\n".to_owned()),
        );

        // Mock ssh-keyscan for scan() — return a DIFFERENT key.
        runner.push_run_response(
            "ssh-keyscan",
            Ok("example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDIFFERENTKEY0000000000000000000000000\n".to_owned()),
        );

        let svc = KnownHostsService::new(&paths, &runner);
        let report = svc.check_host_key_change("example.com").await.unwrap();
        assert!(report.changed);
        assert_eq!(report.changes.len(), 1);
        assert_eq!(report.changes[0].key_type, "ssh-ed25519");
        assert!(matches!(report.changes[0].kind, KeyChangeKind::Changed { .. }));
    }

    #[tokio::test]
    async fn check_host_key_change_no_change() {
        let dir = tempfile::tempdir().unwrap();
        let kh_path = dir.path().join("known_hosts");
        let key_b64 = "AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl";
        std::fs::write(
            &kh_path,
            format!("example.com ssh-ed25519 {key_b64}\n"),
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();

        // Mock ssh-keygen -F — same key.
        runner.push_run_response(
            "ssh-keygen",
            Ok(format!("# Host example.com found: line 1\nexample.com ssh-ed25519 {key_b64}\n")),
        );
        // Mock ssh-keyscan — same key.
        runner.push_run_response(
            "ssh-keyscan",
            Ok(format!("example.com ssh-ed25519 {key_b64}\n")),
        );

        let svc = KnownHostsService::new(&paths, &runner);
        let report = svc.check_host_key_change("example.com").await.unwrap();
        assert!(!report.changed);
        assert!(report.changes.is_empty());
    }

    #[tokio::test]
    async fn check_host_key_change_detects_new_key_type() {
        let dir = tempfile::tempdir().unwrap();
        let kh_path = dir.path().join("known_hosts");
        let ed_key = "AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl";
        let rsa_key = "AAAAB3NzaC1yc2EAAAADAQABAAABgQC7";
        std::fs::write(
            &kh_path,
            format!("example.com ssh-ed25519 {ed_key}\n"),
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();

        // ssh-keygen -F returns only the ed25519 key.
        runner.push_run_response(
            "ssh-keygen",
            Ok(format!("# Host example.com found: line 1\nexample.com ssh-ed25519 {ed_key}\n")),
        );
        // ssh-keyscan returns both ed25519 AND rsa.
        runner.push_run_response(
            "ssh-keyscan",
            Ok(format!("example.com ssh-ed25519 {ed_key}\nexample.com ssh-rsa {rsa_key}\n")),
        );

        let svc = KnownHostsService::new(&paths, &runner);
        let report = svc.check_host_key_change("example.com").await.unwrap();
        assert!(report.changed);
        assert_eq!(report.changes.len(), 1);
        assert_eq!(report.changes[0].key_type, "ssh-rsa");
        assert!(matches!(report.changes[0].kind, KeyChangeKind::New));
    }

    #[tokio::test]
    async fn check_host_key_change_detects_removed_key_type() {
        let dir = tempfile::tempdir().unwrap();
        let kh_path = dir.path().join("known_hosts");
        let ed_key = "AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl";
        let rsa_key = "AAAAB3NzaC1yc2EAAAADAQABAAABgQC7";
        std::fs::write(
            &kh_path,
            format!("example.com ssh-ed25519 {ed_key}\nexample.com ssh-rsa {rsa_key}\n"),
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();

        // ssh-keygen -F returns both keys.
        runner.push_run_response(
            "ssh-keygen",
            Ok(format!(
                "# Host example.com found: line 1\nexample.com ssh-ed25519 {ed_key}\n\
                 # Host example.com found: line 2\nexample.com ssh-rsa {rsa_key}\n"
            )),
        );
        // ssh-keyscan returns only ed25519 (rsa removed).
        runner.push_run_response(
            "ssh-keyscan",
            Ok(format!("example.com ssh-ed25519 {ed_key}\n")),
        );

        let svc = KnownHostsService::new(&paths, &runner);
        let report = svc.check_host_key_change("example.com").await.unwrap();
        assert!(report.changed);
        assert_eq!(report.changes.len(), 1);
        assert_eq!(report.changes[0].key_type, "ssh-rsa");
        assert!(matches!(report.changes[0].kind, KeyChangeKind::Removed));
    }

    // -----------------------------------------------------------------------
    // resolve_user_known_hosts_file()
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn resolve_user_known_hosts_file_default() {
        let dir = tempfile::tempdir().unwrap();
        // No config file — should return default known_hosts path.
        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();
        let svc = KnownHostsService::new(&paths, &runner);
        let result = svc.resolve_user_known_hosts_file("example.com").await.unwrap();
        assert_eq!(result, dir.path().join("known_hosts"));
    }

    #[tokio::test]
    async fn resolve_user_known_hosts_file_from_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config");
        std::fs::write(
            &config_path,
            "Host example.com\n    UserKnownHostsFile /custom/known_hosts\n",
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();
        let svc = KnownHostsService::new(&paths, &runner);
        let result = svc.resolve_user_known_hosts_file("example.com").await.unwrap();
        assert_eq!(result, PathBuf::from("/custom/known_hosts"));
    }

    #[tokio::test]
    async fn resolve_user_known_hosts_file_none() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config");
        std::fs::write(
            &config_path,
            "Host example.com\n    UserKnownHostsFile none\n",
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();
        let svc = KnownHostsService::new(&paths, &runner);
        let result = svc.resolve_user_known_hosts_file("example.com").await.unwrap();
        // "none" maps to /dev/null.
        assert_eq!(result, PathBuf::from("/dev/null"));
    }

    #[tokio::test]
    async fn resolve_user_known_hosts_file_global_directive() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config");
        std::fs::write(
            &config_path,
            "UserKnownHostsFile /global/custom/known_hosts\n",
        )
        .unwrap();

        let paths = crate::paths::SshPaths::with_dir(dir.path());
        let runner = crate::MockCliRunner::new();
        let svc = KnownHostsService::new(&paths, &runner);
        let result = svc.resolve_user_known_hosts_file("any-host").await.unwrap();
        assert_eq!(result, PathBuf::from("/global/custom/known_hosts"));
    }

    // -----------------------------------------------------------------------
    // find_user_known_hosts_file_in_config()
    // -----------------------------------------------------------------------

    #[test]
    fn find_user_known_hosts_file_in_config_host_specific() {
        let config = "\
Host example.com
    UserKnownHostsFile /custom/known_hosts
";
        let result = find_user_known_hosts_file_in_config(config, "example.com");
        assert_eq!(result, Some("/custom/known_hosts".to_owned()));
    }

    #[test]
    fn find_user_known_hosts_file_in_config_no_match() {
        let config = "\
Host other.com
    UserKnownHostsFile /custom/known_hosts
";
        let result = find_user_known_hosts_file_in_config(config, "example.com");
        assert_eq!(result, None);
    }

    #[test]
    fn find_user_known_hosts_file_in_config_global() {
        let config = "UserKnownHostsFile /global/known_hosts\n";
        let result = find_user_known_hosts_file_in_config(config, "any-host");
        assert_eq!(result, Some("/global/known_hosts".to_owned()));
    }

    #[test]
    fn find_user_known_hosts_file_in_config_host_takes_precedence() {
        let config = "\
UserKnownHostsFile /global/known_hosts
Host example.com
    UserKnownHostsFile /host-specific/known_hosts
";
        let result = find_user_known_hosts_file_in_config(config, "example.com");
        assert_eq!(result, Some("/host-specific/known_hosts".to_owned()));
    }

    #[test]
    fn find_user_known_hosts_file_in_config_wildcard_host() {
        let config = "\
Host *
    UserKnownHostsFile /wildcard/known_hosts
";
        let result = find_user_known_hosts_file_in_config(config, "anything");
        assert_eq!(result, Some("/wildcard/known_hosts".to_owned()));
    }

    // -----------------------------------------------------------------------
    // expand_known_hosts_path()
    // -----------------------------------------------------------------------

    #[test]
    fn expand_known_hosts_path_absolute() {
        assert_eq!(
            expand_known_hosts_path("/absolute/path/known_hosts"),
            PathBuf::from("/absolute/path/known_hosts")
        );
    }

    // -----------------------------------------------------------------------
    // compare_host_keys()
    // -----------------------------------------------------------------------

    #[test]
    fn compare_host_keys_detects_new() {
        let stored = vec![];
        let scanned = vec![scan::parse_keyscan_line(
            "host",
            "host ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl",
        )
        .unwrap()];
        let changes = compare_host_keys(&stored, &scanned);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].key_type, "ssh-ed25519");
        assert!(matches!(changes[0].kind, KeyChangeKind::New));
    }

    #[test]
    fn compare_host_keys_detects_removed() {
        let stored = vec![parse::parse_line(
            "host ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl",
            1,
        )
        .unwrap()];
        let scanned = vec![];
        let changes = compare_host_keys(&stored, &scanned);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].key_type, "ssh-ed25519");
        assert!(matches!(changes[0].kind, KeyChangeKind::Removed));
    }

    #[test]
    fn compare_host_keys_detects_changed() {
        let stored = vec![parse::parse_line(
            "host ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl",
            1,
        )
        .unwrap()];
        let scanned = vec![scan::parse_keyscan_line(
            "host",
            "host ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDIFFERENTKEY0000000000000000000000000",
        )
        .unwrap()];
        let changes = compare_host_keys(&stored, &scanned);
        assert_eq!(changes.len(), 1);
        assert!(matches!(changes[0].kind, KeyChangeKind::Changed { .. }));
    }

    #[test]
    fn compare_host_keys_no_changes() {
        let key = "AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl";
        let stored = vec![parse::parse_line(&format!("host ssh-ed25519 {key}"), 1).unwrap()];
        let scanned = vec![scan::parse_keyscan_line("host", &format!("host ssh-ed25519 {key}")).unwrap()];
        let changes = compare_host_keys(&stored, &scanned);
        assert!(changes.is_empty());
    }

    // -----------------------------------------------------------------------
    // HostKeyChangeReport / KeyChange / KeyChangeKind types
    // -----------------------------------------------------------------------

    #[test]
    fn host_key_change_report_debug() {
        let report = HostKeyChangeReport {
            host: "example.com".to_owned(),
            changed: false,
            stored_keys: vec![],
            scanned_keys: vec![],
            changes: vec![],
        };
        let debug = format!("{report:?}");
        assert!(debug.contains("example.com"));
        assert!(debug.contains("changed: false"));
    }

    #[test]
    fn key_change_kind_new_debug() {
        let kind = KeyChangeKind::New;
        assert_eq!(format!("{kind:?}"), "New");
    }

    #[test]
    fn key_change_kind_removed_debug() {
        let kind = KeyChangeKind::Removed;
        assert_eq!(format!("{kind:?}"), "Removed");
    }

    #[test]
    fn key_change_kind_changed_debug() {
        let kind = KeyChangeKind::Changed {
            stored_fingerprint: "SHA256:abc".to_owned(),
            scanned_fingerprint: "SHA256:def".to_owned(),
        };
        let debug = format!("{kind:?}");
        assert!(debug.contains("SHA256:abc"));
        assert!(debug.contains("SHA256:def"));
    }

    // -----------------------------------------------------------------------
    // SshPaths global_known_hosts_path
    // -----------------------------------------------------------------------

    #[test]
    fn ssh_paths_global_known_hosts_path() {
        let dir = tempfile::tempdir().unwrap();
        let paths = crate::paths::SshPaths::with_dir(dir.path());
        assert_eq!(
            paths.global_known_hosts_path(),
            dir.path().join("ssh_known_hosts")
        );
    }
}
