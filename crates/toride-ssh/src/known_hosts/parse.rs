//! Parse `known_hosts` files.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// A single entry parsed from a `known_hosts` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownHostEntry {
    /// Markers such as `@cert-authority` or `@revoked`.
    pub markers: Vec<String>,
    /// Hostname patterns (comma-separated in the file; may include `*`/`?`
    /// globs, `!` negations, `[host]:port` forms, or `|1|...` hashed names).
    pub hosts: Vec<String>,
    /// Key type label, e.g. `ssh-ed25519`.
    pub key_type: String,
    /// Base64-encoded public key blob.
    pub public_key: String,
    /// Optional trailing comment on the line (everything after the base64 key,
    /// excluding any `#`-delimited note).
    pub comment: Option<String>,
    /// 1-based line number within the file.
    pub line_number: usize,
}

/// Parse a known_hosts file at the given path.
///
/// This reads the file asynchronously and parses each non-empty, non-comment
/// line into a [`KnownHostEntry`].  Hashed hostnames (`|1|...`) are preserved
/// as-is in the `hosts` field.
pub async fn parse_known_hosts(path: &Path) -> Result<Vec<KnownHostEntry>> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || parse_known_hosts_sync(&path))
        .await
        .map_err(|e| Error::KnownHostsParseFailed(e.to_string()))?
}

/// Synchronous implementation that does the actual parsing.
fn parse_known_hosts_sync(path: &Path) -> Result<Vec<KnownHostEntry>> {
    let contents = std::fs::read_to_string(path)?;

    let mut entries = Vec::new();

    for (idx, raw_line) in contents.lines().enumerate() {
        let line_number = idx + 1;

        // Lines whose first non-whitespace character is '#' are full-line
        // comments.  Blank lines are skipped.
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // We hand-parse here so we can capture the comment field and preserve
        // the raw host-pattern strings (the ssh-key crate decodes hashed
        // hosts into bytes which is less useful for display).
        match parse_line(trimmed, line_number) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                // Log malformed lines but continue parsing the rest.
                tracing::warn!(
                    line_number,
                    error = %e,
                    "skipping malformed known_hosts line"
                );
            }
        }
    }

    Ok(entries)
}

/// Parse a single trimmed known_hosts line.
///
/// Format: `[markers] host-patterns key-type base64-key [comment]`
///
/// The marker is an optional `@cert-authority` or `@revoked` prefixed token.
/// Only one marker is supported per OpenSSH known_hosts format.
fn parse_line(line: &str, line_number: usize) -> Result<KnownHostEntry> {
    // Skip full-line comments — these are filtered at the sync level,
    // but parse_line should handle them gracefully if called directly.
    if line.trim_start().starts_with('#') {
        return Err(Error::KnownHostsParseFailed(format!(
            "line {line_number}: comment"
        )));
    }

    // 1. Detect optional marker (starts with '@').
    let (markers, rest) = if line.starts_with('@') {
        let (marker_str, rest) = line
            .split_once(' ')
            .ok_or_else(|| Error::KnownHostsParseFailed(format!(
                "line {line_number}: marker without trailing fields"
            )))?;
        (vec![marker_str.to_owned()], rest)
    } else {
        (vec![], line)
    };

    // 2. Split into remaining whitespace-separated fields.
    //    We expect at least: hosts  keytype  base64key
    let fields: Vec<&str> = rest.split_whitespace().collect();
    if fields.len() < 3 {
        return Err(Error::KnownHostsParseFailed(format!(
            "line {line_number}: expected at least 3 fields (hosts, key-type, key), got {}",
            fields.len()
        )));
    }

    let hosts_str = fields[0];
    let hosts: Vec<String> = hosts_str.split_terminator(',').map(String::from).collect();

    let key_type = fields[1].to_owned();
    let public_key = fields[2].to_owned();

    let comment = if fields.len() > 3 {
        Some(fields[3..].join(" "))
    } else {
        None
    };

    Ok(KnownHostEntry {
        markers,
        hosts,
        key_type,
        public_key,
        comment,
        line_number,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_entry() {
        let line = "github.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl";
        let entry = parse_line(line, 1).unwrap();
        assert!(entry.markers.is_empty());
        assert_eq!(entry.hosts, vec!["github.com"]);
        assert_eq!(entry.key_type, "ssh-ed25519");
        assert!(entry.comment.is_none());
    }

    #[test]
    fn parse_entry_with_comment() {
        let line = "example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIB9dG4kjRhQTtWTVzd2t27+t0DEHBPW7iOD23TUiYLio my comment here";
        let entry = parse_line(line, 2).unwrap();
        assert_eq!(entry.comment.as_deref(), Some("my comment here"));
    }

    #[test]
    fn parse_entry_with_marker() {
        let line = "@revoked example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIB9dG4kjRhQTtWTVzd2t27+t0DEHBPW7iOD23TUiYLio";
        let entry = parse_line(line, 3).unwrap();
        assert_eq!(entry.markers, vec!["@revoked"]);
    }

    #[test]
    fn parse_cert_authority_marker() {
        let line = "@cert-authority *.example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIB9dG4kjRhQTtWTVzd2t27+t0DEHBPW7iOD23TUiYLio";
        let entry = parse_line(line, 10).unwrap();
        assert_eq!(entry.markers, vec!["@cert-authority"]);
        assert_eq!(entry.hosts, vec!["*.example.com"]);
    }

    #[test]
    fn parse_hashed_host() {
        let line = "|1|JfKTdBh7rNbXkVAQCRp4OQoPfmI=|USECr3SWf1JUPsms5AqfD5QfxkM= ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIB9dG4kjRhQTtWTVzd2t27+t0DEHBPW7iOD23TUiYLio";
        let entry = parse_line(line, 4).unwrap();
        assert_eq!(entry.hosts.len(), 1);
        assert!(entry.hosts[0].starts_with("|1|"));
    }

    #[test]
    fn parse_multiple_hosts() {
        let line = "host1,host2,!host3 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIB9dG4kjRhQTtWTVzd2t27+t0DEHBPW7iOD23TUiYLio";
        let entry = parse_line(line, 5).unwrap();
        assert_eq!(entry.hosts, vec!["host1", "host2", "!host3"]);
    }

    #[test]
    fn parse_bracketed_host_port() {
        let line = "[192.168.1.1]:2222 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIB9dG4kjRhQTtWTVzd2t27+t0DEHBPW7iOD23TUiYLio";
        let entry = parse_line(line, 6).unwrap();
        assert_eq!(entry.hosts, vec!["[192.168.1.1]:2222"]);
    }

    #[test]
    fn parse_ipv6_bracketed() {
        let line = "[::1]:22 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIB9dG4kjRhQTtWTVzd2t27+t0DEHBPW7iOD23TUiYLio";
        let entry = parse_line(line, 7).unwrap();
        assert_eq!(entry.hosts, vec!["[::1]:22"]);
    }

    #[test]
    fn reject_insufficient_fields() {
        // Only hosts field, no key type or key.
        assert!(parse_line("github.com", 1).is_err());
        // Hosts + key type, but no base64 key.
        assert!(parse_line("github.com ssh-ed25519", 2).is_err());
    }

    #[test]
    fn full_line_comment_is_skipped() {
        // This would be handled at the sync level (line starts with '#'),
        // but parse_line should still work if called directly.
        // It will fail because '#' has no 3 fields, which is correct.
        assert!(parse_line("# this is a comment", 8).is_err());
    }

    #[test]
    fn parse_line_with_rsa_key() {
        let line = "host ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQC7";
        let entry = parse_line(line, 9).unwrap();
        assert_eq!(entry.key_type, "ssh-rsa");
        assert_eq!(entry.public_key, "AAAAB3NzaC1yc2EAAAADAQABAAABgQC7");
    }

    #[test]
    fn parse_line_with_ecdsa_key() {
        let line = "host ecdsa-sha2-nistp256 AAAAE2VjZHNhLXNoYTItbmlzdHAyNTY=";
        let entry = parse_line(line, 10).unwrap();
        assert_eq!(entry.key_type, "ecdsa-sha2-nistp256");
    }

    #[test]
    fn parse_line_with_sk_key() {
        let line = "host sk-ssh-ed25519@openssh.com AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl";
        let entry = parse_line(line, 11).unwrap();
        assert_eq!(entry.key_type, "sk-ssh-ed25519@openssh.com");
    }
}
