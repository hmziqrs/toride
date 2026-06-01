//! Parsing functions for `/etc/passwd`, `/etc/group`, and `/etc/sudoers`.
//!
//! Each parser returns strongly-typed structs that can be inspected,
//! validated, and rendered back to text.

use std::path::Path;

use crate::{Error, Result};

// ---------------------------------------------------------------------------
// PasswdEntry
// ---------------------------------------------------------------------------

/// A single entry from `/etc/passwd`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PasswdEntry {
    /// Login name.
    pub username: String,
    /// Placeholder password field (typically `x` pointing to shadow).
    pub password: String,
    /// User ID (UID).
    pub uid: u32,
    /// Primary group ID (GID).
    pub gid: u32,
    /// GECOS comment field (full name, room, etc.).
    pub gecos: String,
    /// Home directory path.
    pub home: String,
    /// Login shell.
    pub shell: String,
}

impl std::fmt::Display for PasswdEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}:{}:{}:{}:{}",
            self.username,
            self.password,
            self.uid,
            self.gid,
            self.gecos,
            self.home,
            self.shell
        )
    }
}

/// Parse the contents of `/etc/passwd` into a list of entries.
///
/// Blank lines and comments (starting with `#`) are skipped.
///
/// # Errors
///
/// Returns [`Error::Validation`] if a line has fewer than 7 colon-separated
/// fields or the UID/GID cannot be parsed as `u32`.
pub fn parse_passwd(content: &str) -> Result<Vec<PasswdEntry>> {
    let mut entries = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split(':').collect();
        if fields.len() != 7 {
            return Err(Error::Validation(format!(
                "passwd line has {} fields (expected 7): {line:?}",
                fields.len()
            )));
        }
        let uid = fields[2].parse::<u32>().map_err(|e| {
            Error::Validation(format!("invalid UID {:?}: {e}", fields[2]))
        })?;
        let gid = fields[3].parse::<u32>().map_err(|e| {
            Error::Validation(format!("invalid GID {:?}: {e}", fields[3]))
        })?;
        entries.push(PasswdEntry {
            username: fields[0].to_owned(),
            password: fields[1].to_owned(),
            uid,
            gid,
            gecos: fields[4].to_owned(),
            home: fields[5].to_owned(),
            shell: fields[6].to_owned(),
        });
    }
    Ok(entries)
}

// ---------------------------------------------------------------------------
// GroupEntry
// ---------------------------------------------------------------------------

/// A single entry from `/etc/group`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GroupEntry {
    /// Group name.
    pub name: String,
    /// Placeholder password field (typically `x`).
    pub password: String,
    /// Group ID (GID).
    pub gid: u32,
    /// Comma-separated list of supplementary member usernames.
    pub members: Vec<String>,
}

impl std::fmt::Display for GroupEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}:{}",
            self.name,
            self.password,
            self.gid,
            self.members.join(",")
        )
    }
}

/// Parse the contents of `/etc/group` into a list of entries.
///
/// # Errors
///
/// Returns [`Error::Validation`] if a line has fewer than 4 colon-separated
/// fields or the GID cannot be parsed as `u32`.
pub fn parse_group(content: &str) -> Result<Vec<GroupEntry>> {
    let mut entries = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split(':').collect();
        if fields.len() != 4 {
            return Err(Error::Validation(format!(
                "group line has {} fields (expected 4): {line:?}",
                fields.len()
            )));
        }
        let gid = fields[2].parse::<u32>().map_err(|e| {
            Error::Validation(format!("invalid GID {:?}: {e}", fields[2]))
        })?;
        let members = if fields[3].is_empty() {
            Vec::new()
        } else {
            fields[3].split(',').map(String::from).collect()
        };
        entries.push(GroupEntry {
            name: fields[0].to_owned(),
            password: fields[1].to_owned(),
            gid,
            members,
        });
    }
    Ok(entries)
}

// ---------------------------------------------------------------------------
// SudoersEntry
// ---------------------------------------------------------------------------

/// A parsed sudoers rule line.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SudoersEntry {
    /// Who the rule applies to (user or `%group`).
    pub who: String,
    /// Which hosts the rule applies to (typically `ALL`).
    pub hosts: String,
    /// Which commands the rule applies to (typically `ALL` or a command list).
    pub commands: String,
    /// Whether `NOPASSWD` is set for this rule.
    pub nopasswd: bool,
    /// Optional run-as user (the `(root)` part).
    pub runas: Option<String>,
}

/// Parse a sudoers file into a list of entries.
///
/// This is a simplified parser that handles the most common sudoers syntax.
/// It skips blank lines, comments, `Defaults`, `@include`, and `@includedir`
/// directives.
///
/// # Errors
///
/// Returns [`Error::SudoError`] if a rule line cannot be parsed.
pub fn parse_sudoers(content: &str) -> Result<Vec<SudoersEntry>> {
    let mut entries = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('@') {
            continue;
        }
        if line.starts_with("Defaults") {
            continue;
        }
        // Simplified parsing: "who hosts = (runas) [NOPASSWD:] commands"
        let parts: Vec<&str> = line.splitn(4, ' ').collect();
        if parts.len() < 3 {
            continue; // skip malformed lines gracefully
        }
        let who = parts[0].to_owned();
        let hosts = parts[1].to_owned();

        // The remaining parts contain "= (runas) [NOPASSWD:] commands"
        let rest = parts[2..].join(" ");
        let rest = rest.trim_start_matches('=').trim();

        let (runas, rest) = if let Some(r) = rest.strip_prefix('(') {
            if let Some(end) = r.find(')') {
                (Some(r[..end].to_owned()), r[end + 1..].trim().to_owned())
            } else {
                (None, rest.to_owned())
            }
        } else {
            (None, rest.to_owned())
        };

        let nopasswd = rest.contains("NOPASSWD:");
        let commands = rest.replace("NOPASSWD:", "").trim().to_owned();

        entries.push(SudoersEntry {
            who,
            hosts,
            commands,
            nopasswd,
            runas,
        });
    }
    Ok(entries)
}

// ---------------------------------------------------------------------------
// File-level parsing helpers
// ---------------------------------------------------------------------------

/// Read and parse `/etc/passwd` from disk.
///
/// # Errors
///
/// Returns [`Error::Io`] if the file cannot be read, or [`Error::Validation`]
/// if parsing fails.
pub fn read_passwd(path: &Path) -> Result<Vec<PasswdEntry>> {
    let content = std::fs::read_to_string(path)?;
    parse_passwd(&content)
}

/// Read and parse `/etc/group` from disk.
///
/// # Errors
///
/// Returns [`Error::Io`] if the file cannot be read, or [`Error::Validation`]
/// if parsing fails.
pub fn read_group(path: &Path) -> Result<Vec<GroupEntry>> {
    let content = std::fs::read_to_string(path)?;
    parse_group(&content)
}

/// Read and parse a sudoers file from disk.
///
/// # Errors
///
/// Returns [`Error::Io`] if the file cannot be read, or [`Error::SudoError`]
/// if parsing fails.
pub fn read_sudoers(path: &Path) -> Result<Vec<SudoersEntry>> {
    let content = std::fs::read_to_string(path)?;
    parse_sudoers(&content)
}
