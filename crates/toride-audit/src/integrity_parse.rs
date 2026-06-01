//! AIDE output parsing.
//!
//! Provides parsers for AIDE check and diff output, converting raw text
//! into structured types for programmatic consumption.

// ---------------------------------------------------------------------------
// AideChange
// ---------------------------------------------------------------------------

/// A single file change detected by AIDE.
#[derive(Debug, Clone)]
pub struct AideChange {
    /// The path of the changed file.
    pub path: String,
    /// The type of change detected.
    pub change_type: AideChangeType,
    /// Details about the change (old/new values).
    pub details: Vec<String>,
}

// ---------------------------------------------------------------------------
// AideChangeType
// ---------------------------------------------------------------------------

/// The type of change detected by AIDE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AideChangeType {
    /// A new file was added.
    Added,
    /// A file was removed.
    Removed,
    /// A file was modified.
    Changed,
    /// The change type could not be determined.
    Unknown,
}

// ---------------------------------------------------------------------------
// AideCheckResult
// ---------------------------------------------------------------------------

/// Parsed result of an AIDE integrity check.
#[derive(Debug, Clone)]
pub struct AideCheckResult {
    /// Whether the check passed (no changes detected).
    pub passed: bool,
    /// Number of files added.
    pub added: usize,
    /// Number of files removed.
    pub removed: usize,
    /// Number of files changed.
    pub changed: usize,
    /// Individual changes detected.
    pub changes: Vec<AideChange>,
    /// Raw output from AIDE.
    pub raw_output: String,
}

// ---------------------------------------------------------------------------
// Parsing functions
// ---------------------------------------------------------------------------

/// Parse AIDE check output into a structured result.
///
/// Attempts to extract summary counts from the AIDE output. Falls back
/// to scanning for change indicators if the summary line is not found.
///
/// # Errors
///
/// Returns [`crate::Error::AideError`] if the output cannot be parsed.
pub fn parse_aide_check(output: &str) -> crate::Result<AideCheckResult> {
    let mut result = AideCheckResult {
        passed: true,
        added: 0,
        removed: 0,
        changed: 0,
        changes: Vec::new(),
        raw_output: output.to_owned(),
    };

    for line in output.lines() {
        let trimmed = line.trim();

        // Look for summary line like "Changed entries: 5"
        if trimmed.starts_with("Changed entries:") {
            if let Some(num) = extract_number(trimmed) {
                result.changed = num;
                if num > 0 {
                    result.passed = false;
                }
            }
        } else if trimmed.starts_with("Added entries:") {
            if let Some(num) = extract_number(trimmed) {
                result.added = num;
                if num > 0 {
                    result.passed = false;
                }
            }
        } else if trimmed.starts_with("Removed entries:") {
            if let Some(num) = extract_number(trimmed) {
                result.removed = num;
                if num > 0 {
                    result.passed = false;
                }
            }
        }

        // Detect individual changes.
        if trimmed.starts_with("f = ") || trimmed.starts_with("f+++ ") {
            result.changes.push(AideChange {
                path: trimmed[4..].trim().to_owned(),
                change_type: AideChangeType::Added,
                details: Vec::new(),
            });
        } else if trimmed.starts_with("f--- ") {
            result.changes.push(AideChange {
                path: trimmed[5..].trim().to_owned(),
                change_type: AideChangeType::Removed,
                details: Vec::new(),
            });
        } else if trimmed.starts_with("f!! ") || trimmed.starts_with("f>p ") {
            result.changes.push(AideChange {
                path: trimmed[4..].trim().to_owned(),
                change_type: AideChangeType::Changed,
                details: Vec::new(),
            });
        }
    }

    Ok(result)
}

/// Parse AIDE database initialization output.
///
/// Returns the raw output as a result, checking for success indicators.
pub fn parse_aide_init(output: &str) -> crate::Result<bool> {
    Ok(output.contains("AIDE database initialized"))
}

/// Extract a trailing number from a string like "Changed entries: 5".
fn extract_number(s: &str) -> Option<usize> {
    s.split_whitespace()
        .last()
        .and_then(|v| v.parse().ok())
}
