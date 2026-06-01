//! Parsing functions for sysctl output and configuration files.
//!
//! Handles `sysctl -a` output, `/etc/sysctl.conf` content, and
//! `findmnt` output for shared memory mounts.

use crate::error::{Error, Result};
use crate::spec::SysctlParam;

/// Information about a mounted filesystem, parsed from `findmnt` output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountInfo {
    /// Mount target path, e.g. `/dev/shm`.
    pub target: String,
    /// Filesystem type, e.g. `tmpfs`.
    pub fstype: String,
    /// Mount options as a comma-separated string.
    pub options: String,
    /// Source device, e.g. `tmpfs` or `none`.
    pub source: String,
}

/// Parse `sysctl -a` or `sysctl <key>` output into key-value pairs.
///
/// Input format: `key = value` (one per line). Blank lines and comments
/// (starting with `#` or `;`) are skipped.
///
/// # Example
///
/// ```
/// use toride_harden::parse::parse_sysctl_output;
///
/// let output = "kernel.kptr_restrict = 1\nkernel.aslr = 2\n";
/// let pairs = parse_sysctl_output(output);
/// assert_eq!(pairs.len(), 2);
/// assert_eq!(pairs[0], ("kernel.kptr_restrict".into(), "1".into()));
/// ```
pub fn parse_sysctl_output(output: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();

    for line in output.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        // sysctl output uses " = " as separator
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            if !key.is_empty() {
                pairs.push((key, value));
            }
        }
    }

    pairs
}

/// Parse `/etc/sysctl.conf` content into typed [`SysctlParam`] values.
///
/// Lines starting with `#` are comments. Active lines follow the format:
/// `key = value` or `key=value`.
///
/// Description is extracted from the preceding comment line if it starts
/// with `# description:`.
pub fn parse_sysctl_conf(content: &str) -> Vec<SysctlParam> {
    let mut params = Vec::new();
    let mut last_description = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            last_description.clear();
            continue;
        }

        // Comment lines
        if trimmed.starts_with('#') {
            let comment = trimmed.trim_start_matches('#').trim();
            if let Some(desc) = comment.strip_prefix("description:") {
                last_description = desc.trim().to_string();
            } else {
                last_description.clear();
            }
            continue;
        }

        // Key = Value lines
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            if !key.is_empty() {
                params.push(SysctlParam::new(
                    key,
                    value,
                    std::mem::take(&mut last_description),
                ));
            }
        }
    }

    params
}

/// Parse `findmnt` output into [`MountInfo`] values.
///
/// Expected input format (default `findmnt` list output):
/// ```text
/// TARGET    SOURCE   FSTYPE  OPTIONS
/// /dev/shm  tmpfs    tmpfs   rw,nosuid,nodev,noexec
/// ```
///
/// The first line is assumed to be a header and is skipped.
pub fn parse_findmnt_output(output: &str) -> Vec<MountInfo> {
    let mut mounts = Vec::new();
    let mut lines = output.lines();

    // Skip header line
    lines.next();

    for line in lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        mounts.push(MountInfo {
            target: parts[0].to_string(),
            source: parts[1].to_string(),
            fstype: parts[2].to_string(),
            options: parts[3].to_string(),
        });
    }

    mounts
}

/// Parse a single sysctl value output (e.g. from `sysctl -n kernel.kptr_restrict`).
///
/// Returns the trimmed value string, or an error if the output is empty.
pub fn parse_single_value(output: &str) -> Result<String> {
    let value = output.trim().to_string();
    if value.is_empty() {
        return Err(Error::SysctlParse("empty value returned".into()));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sysctl_output_basic() {
        let output = "kernel.kptr_restrict = 1\nkernel.aslr = 2\n# comment\n";
        let pairs = parse_sysctl_output(output);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].0, "kernel.kptr_restrict");
        assert_eq!(pairs[0].1, "1");
    }

    #[test]
    fn parse_sysctl_output_skips_empty_and_comments() {
        let output = "\n# header\n; inline comment\nnet.ipv4.ip_forward = 0\n";
        let pairs = parse_sysctl_output(output);
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn parse_sysctl_conf_with_descriptions() {
        let content = "# description: Restrict kernel pointers\nkernel.kptr_restrict = 1\n";
        let params = parse_sysctl_conf(content);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].key, "kernel.kptr_restrict");
        assert_eq!(params[0].description, "Restrict kernel pointers");
    }

    #[test]
    fn parse_sysctl_conf_ignores_commented_params() {
        let content = "# kernel.kptr_restrict = 1\nkernel.aslr = 2\n";
        let params = parse_sysctl_conf(content);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].key, "kernel.aslr");
    }

    #[test]
    fn parse_findmnt_output() {
        let output = "TARGET    SOURCE   FSTYPE  OPTIONS\n/dev/shm  tmpfs    tmpfs   rw,nosuid,nodev,noexec\n";
        let mounts = parse_findmnt_output(output);
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].target, "/dev/shm");
        assert_eq!(mounts[0].options, "rw,nosuid,nodev,noexec");
    }

    #[test]
    fn parse_single_value_trims_whitespace() {
        let val = parse_single_value("  42  \n").unwrap();
        assert_eq!(val, "42");
    }

    #[test]
    fn parse_single_value_rejects_empty() {
        assert!(parse_single_value("  \n").is_err());
    }
}
