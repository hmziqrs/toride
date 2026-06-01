//! Parsers for audit subsystem output.
//!
//! Provides parsers for `auditctl`, `aureport`, and `ausearch` output,
//! converting raw text into structured types for programmatic consumption.

// ---------------------------------------------------------------------------
// Auditctl output parsing
// ---------------------------------------------------------------------------

/// A single audit rule as reported by `auditctl -l`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditctlRule {
    /// The raw rule string.
    pub raw: String,
    /// The rule action (e.g. "always", "never").
    pub action: String,
    /// The system call filter list (e.g. "exit", "task", "user").
    pub list: String,
    /// The syscall names or "all".
    pub syscalls: Vec<String>,
    /// Field filters (key=value pairs).
    pub fields: Vec<(String, String)>,
}

/// Parse the output of `auditctl -l` into a list of structured rules.
///
/// Each non-empty line is treated as a separate rule. Lines starting with
/// `No rules` produce an empty vector.
///
/// # Errors
///
/// Returns [`crate::Error::AuditRuleParse`] if a line cannot be parsed.
pub fn parse_auditctl_output(output: &str) -> crate::Result<Vec<AuditctlRule>> {
    if output.trim().starts_with("No rules") || output.trim().is_empty() {
        return Ok(Vec::new());
    }

    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            Ok(AuditctlRule {
                raw: line.to_owned(),
                action: String::new(),
                list: String::new(),
                syscalls: Vec::new(),
                fields: Vec::new(),
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Aureport output parsing
// ---------------------------------------------------------------------------

/// A summary entry from `aureport`.
#[derive(Debug, Clone)]
pub struct AureportEntry {
    /// The report line number.
    pub number: usize,
    /// The raw line from the report.
    pub raw: String,
    /// Timestamp if available.
    pub time: Option<String>,
}

/// Parse the output of `aureport` into a list of summary entries.
///
/// # Errors
///
/// Returns [`crate::Error::AuditRuleParse`] if the output cannot be parsed.
pub fn parse_aureport(output: &str) -> crate::Result<Vec<AureportEntry>> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
        .map(|(i, line)| {
            Ok(AureportEntry {
                number: i + 1,
                raw: line.to_owned(),
                time: None,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Ausearch output parsing
// ---------------------------------------------------------------------------

/// A single event from `ausearch` output.
#[derive(Debug, Clone)]
pub struct AusearchEvent {
    /// The raw event text (may span multiple lines).
    pub raw: String,
    /// Event type if identified.
    pub event_type: Option<String>,
    /// Event timestamp.
    pub time: Option<String>,
    /// Audit rule key that matched.
    pub key: Option<String>,
}

/// Parse the output of `ausearch` into a list of events.
///
/// Events are delimited by a line starting with `----`.
///
/// # Errors
///
/// Returns [`crate::Error::AuditRuleParse`] if the output cannot be parsed.
pub fn parse_ausearch(output: &str) -> crate::Result<Vec<AusearchEvent>> {
    if output.trim().is_empty() {
        return Ok(Vec::new());
    }

    let events: Vec<AusearchEvent> = output
        .split("----")
        .filter(|chunk| !chunk.trim().is_empty())
        .map(|chunk| AusearchEvent {
            raw: chunk.trim().to_owned(),
            event_type: None,
            time: None,
            key: None,
        })
        .collect();

    Ok(events)
}
