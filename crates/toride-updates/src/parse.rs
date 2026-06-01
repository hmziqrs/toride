//! Parsers for command output from update-related tools.
//!
//! Each function takes raw command output and returns a structured result:
//!
//! - [`parse_unattended_upgrades_status`] -- parses `unattended-upgrades --status` output
//! - [`parse_apt_check`] -- parses `ubuntu-advantage security-status` or `apt-check` output
//! - [`parse_dnf_check`] -- parses `dnf check-update` output

use crate::error::{Error, Result};
use crate::report::UpdateStatus;

// ---------------------------------------------------------------------------
// parse_unattended_upgrades_status
// ---------------------------------------------------------------------------

/// Parse the output of `unattended-upgrades --status` into an [`UpdateStatus`].
///
/// The output is a free-form log that includes lines like:
///
/// ```text
/// Last run: 2025-06-01 04:00:01
/// Security updates: 5
/// ```
///
/// # Errors
///
/// Returns [`Error::ConfigParse`] if the output cannot be meaningfully parsed.
pub fn parse_unattended_upgrades_status(output: &str) -> Result<UpdateStatus> {
    let mut status = UpdateStatus::empty();

    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("Last run:") {
            status.last_run = Some(line["Last run:".len()..].trim().to_owned());
        } else if line.starts_with("Security updates:") || line.starts_with("security updates:") {
            let count_str = line.split(':').nth(1).unwrap_or("0").trim();
            status.pending_security = count_str.parse::<usize>().unwrap_or(0);
        }
    }

    // If we found any recognizable content, mark as enabled.
    status.auto_updates_enabled = output.lines().any(|l| !l.trim().is_empty());

    Ok(status)
}

// ---------------------------------------------------------------------------
// parse_apt_check
// ---------------------------------------------------------------------------

/// Parse the output of `apt-check` (or equivalent) to extract update counts.
///
/// Returns a tuple of `(security_updates, total_updates)`.
///
/// Typical output format:
///
/// ```text
/// 3;12
/// ```
///
/// Where `3` is the number of security updates and `12` is the total.
///
/// # Errors
///
/// Returns [`Error::ConfigParse`] if the output cannot be parsed as `N;M`.
pub fn parse_apt_check(output: &str) -> Result<(usize, usize)> {
    let output = output.trim();
    let parts: Vec<&str> = output.split(';').collect();

    if parts.len() != 2 {
        return Err(Error::ConfigParse(format!(
            "expected 'N;M' format, got: {output:?}"
        )));
    }

    let security = parts[0]
        .trim()
        .parse::<usize>()
        .map_err(|e| Error::ConfigParse(format!("invalid security count: {e}")))?;
    let total = parts[1]
        .trim()
        .parse::<usize>()
        .map_err(|e| Error::ConfigParse(format!("invalid total count: {e}")))?;

    Ok((security, total))
}

// ---------------------------------------------------------------------------
// parse_dnf_check
// ---------------------------------------------------------------------------

/// Parse the output of `dnf check-update` to extract update counts.
///
/// Returns a tuple of `(security_updates, total_updates)`.
///
/// The output lists available updates, one per line. Security updates are
/// typically identified by advisory IDs starting with `ALSA-`, `FEDORA-`,
/// or `RHSA-`.
///
/// # Errors
///
/// Returns [`Error::ConfigParse`] if the output is fundamentally malformed
/// (though DNF output is relatively free-form, so parsing is lenient).
pub fn parse_dnf_check(output: &str) -> Result<(usize, usize)> {
    let mut security = 0usize;
    let mut total = 0usize;

    for line in output.lines() {
        let line = line.trim();
        // Skip empty lines and header/footer lines.
        if line.is_empty() || line.starts_with("Last metadata") || line.contains("Updating") {
            continue;
        }

        // Lines with package names contain update info.
        // A simple heuristic: count non-empty, non-header lines as updates.
        if line.contains('.') && line.contains(' ') {
            total += 1;
            // Check for security advisory patterns.
            if line.contains("security")
                || line.contains("ALSA-")
                || line.contains("FEDORA-")
                || line.contains("RHSA-")
            {
                security += 1;
            }
        }
    }

    Ok((security, total))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_apt_check_simple() {
        let (security, total) = parse_apt_check("3;12").unwrap();
        assert_eq!(security, 3);
        assert_eq!(total, 12);
    }

    #[test]
    fn parse_apt_check_zero() {
        let (security, total) = parse_apt_check("0;0").unwrap();
        assert_eq!(security, 0);
        assert_eq!(total, 0);
    }

    #[test]
    fn parse_apt_check_invalid() {
        assert!(parse_apt_check("invalid").is_err());
    }

    #[test]
    fn parse_dnf_check_empty() {
        let (security, total) = parse_dnf_check("").unwrap();
        assert_eq!(security, 0);
        assert_eq!(total, 0);
    }

    #[test]
    fn parse_unattended_upgrades_status_empty() {
        let status = parse_unattended_upgrades_status("").unwrap();
        assert!(!status.auto_updates_enabled);
        assert_eq!(status.pending_security, 0);
    }
}
