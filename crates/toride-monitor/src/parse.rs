//! Parsers for iptables log output, conntrack entries, and `ss` output.
//!
//! Each parser takes raw string output from system commands and produces
//! structured data types. Parsers are intentionally lenient: unparseable lines
//! are skipped rather than causing hard errors.

use std::net::IpAddr;

use crate::report::ConnectionInfo;
use crate::{Error, Result};

// ---------------------------------------------------------------------------
// IptablesLogEntry
// ---------------------------------------------------------------------------

/// A single entry parsed from iptables LOG target output.
#[derive(Debug, Clone)]
pub struct IptablesLogEntry {
    /// Log prefix (e.g. `"TORIDE_OUT"`).
    pub prefix: String,
    /// Source IP address.
    pub src: IpAddr,
    /// Destination IP address.
    pub dst: IpAddr,
    /// Protocol (e.g. `"TCP"`, `"UDP"`).
    pub proto: String,
    /// Source port, if applicable.
    pub spt: Option<u16>,
    /// Destination port, if applicable.
    pub dpt: Option<u16>,
}

// ---------------------------------------------------------------------------
// ConntrackEntry
// ---------------------------------------------------------------------------

/// A single entry parsed from `conntrack -L` output.
#[derive(Debug, Clone)]
pub struct ConntrackEntry {
    /// Protocol number.
    pub proto: u8,
    /// Source IP address.
    pub src: IpAddr,
    /// Destination IP address.
    pub dst: IpAddr,
    /// Source port.
    pub sport: Option<u16>,
    /// Destination port.
    pub dport: Option<u16>,
    /// Connection state (e.g. `"ESTABLISHED"`).
    pub state: Option<String>,
    /// Bytes transferred.
    pub bytes: Option<u64>,
    /// Packets transferred.
    pub packets: Option<u64>,
}

// ---------------------------------------------------------------------------
// SsEntry
// ---------------------------------------------------------------------------

/// A single entry parsed from `ss -tunap` output.
#[derive(Debug, Clone)]
pub struct SsEntry {
    /// Network ID (e.g. `"tcp"`, `"udp"`).
    pub netid: String,
    /// Connection state (e.g. `"ESTAB"`, `"TIME-WAIT"`).
    pub state: String,
    /// Local address (IP:port).
    pub local: String,
    /// Peer address (IP:port).
    pub peer: String,
    /// Process information, if available.
    pub process: Option<String>,
}

// ---------------------------------------------------------------------------
// Parsing functions
// ---------------------------------------------------------------------------

/// Parse iptables LOG target output into structured entries.
///
/// Input is typically read from the kernel ring buffer (`dmesg` or
/// `/var/log/kern.log`) and filtered by the configured log prefix.
///
/// # Errors
///
/// Returns [`Error::LoggingError`] only for fundamentally malformed input
/// (e.g. empty after stripping). Individual unparseable lines are skipped.
pub fn parse_iptables_log(input: &str, prefix: &str) -> Result<Vec<IptablesLogEntry>> {
    let mut entries = Vec::new();

    for line in input.lines() {
        if !line.contains(prefix) {
            continue;
        }

        // Attempt to extract fields from a typical iptables LOG line:
        // ... PREFIX IN= OUT=eth0 SRC=1.2.3.4 DST=5.6.7.8 PROTO=TCP SPT=12345 DPT=80
        let entry = parse_single_iptables_line(line, prefix);
        if let Some(e) = entry {
            entries.push(e);
        }
    }

    Ok(entries)
}

/// Parse a single iptables log line into an [`IptablesLogEntry`].
///
/// Returns `None` for lines that cannot be fully parsed rather than
/// producing an error.
fn parse_single_iptables_line(line: &str, _prefix: &str) -> Option<IptablesLogEntry> {
    let prefix = extract_field(line, "PREFIX=").unwrap_or_default();
    let src = extract_field(line, "SRC=")?.parse().ok()?;
    let dst = extract_field(line, "DST=")?.parse().ok()?;
    let proto = extract_field(line, "PROTO=").unwrap_or_else(|| "UNKNOWN".to_owned());
    let spt = extract_field(line, "SPT=").and_then(|s| s.parse().ok());
    let dpt = extract_field(line, "DPT=").and_then(|s| s.parse().ok());

    Some(IptablesLogEntry {
        prefix,
        src,
        dst,
        proto,
        spt,
        dpt,
    })
}

/// Parse `conntrack -L` output into structured entries.
///
/// # Errors
///
/// Returns [`Error::ConntrackError`] for fundamentally malformed input.
/// Individual unparseable lines are skipped.
pub fn parse_conntrack_output(input: &str) -> Result<Vec<ConntrackEntry>> {
    let mut entries = Vec::new();

    for line in input.lines() {
        if let Some(entry) = parse_single_conntrack_line(line) {
            entries.push(entry);
        }
    }

    Ok(entries)
}

/// Parse a single conntrack line.
fn parse_single_conntrack_line(line: &str) -> Option<ConntrackEntry> {
    // conntrack output format:
    // tcp  6 431998 ESTABLISHED src=1.2.3.4 dst=5.6.7.8 sport=12345 dport=80 bytes=1234 packets=56

    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let proto = parts.first().and_then(|s| match *s {
        "tcp" => Some(6),
        "udp" => Some(17),
        "icmp" => Some(1),
        _ => None,
    })?;

    let src = extract_field(line, "src=")?.parse().ok()?;
    let dst = extract_field(line, "dst=")?.parse().ok()?;
    let sport = extract_field(line, "sport=").and_then(|s| s.parse().ok());
    let dport = extract_field(line, "dport=").and_then(|s| s.parse().ok());
    let state = extract_field(line, "");
    let bytes = extract_field(line, "bytes=").and_then(|s| s.parse().ok());
    let packets = extract_field(line, "packets=").and_then(|s| s.parse().ok());

    Some(ConntrackEntry {
        proto,
        src,
        dst,
        sport,
        dport,
        state,
        bytes,
        packets,
    })
}

/// Parse `ss -tunap` output into structured entries.
///
/// # Errors
///
/// Returns [`Error::ConntrackError`] for fundamentally malformed input.
/// Individual unparseable lines are skipped.
pub fn parse_ss_output(input: &str) -> Result<Vec<SsEntry>> {
    let mut entries = Vec::new();
    let mut lines = input.lines();

    // Skip the header line.
    lines.next();

    for line in lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        let netid = parts.first().unwrap().to_string();
        let state = parts.get(1).unwrap().to_string();
        let local = parts.get(2).unwrap().to_string();
        let peer = parts.get(3).unwrap().to_string();
        let process = parts.get(4).map(|s| (*s).to_string());

        entries.push(SsEntry {
            netid,
            state,
            local,
            peer,
            process,
        });
    }

    Ok(entries)
}

/// Convert an [`SsEntry`] into a [`ConnectionInfo`].
///
/// Returns `None` if the address cannot be parsed.
pub fn ss_entry_to_connection(entry: &SsEntry) -> Option<ConnectionInfo> {
    let (src, src_port) = parse_addr_port(&entry.local)?;
    let (dst, dst_port) = parse_addr_port(&entry.peer)?;

    Some(ConnectionInfo {
        src,
        src_port,
        dst,
        dst_port,
        protocol: entry.netid.to_lowercase(),
        state: entry.state.clone(),
        bytes: None,
        packets: None,
    })
}

/// Parse an `ip:port` or `[ipv6]:port` string.
fn parse_addr_port(s: &str) -> Option<(IpAddr, u16)> {
    let (ip_str, port_str) = if s.starts_with('[') {
        // IPv6: [::1]:12345
        let close = s.find(']')?;
        let ip = &s[1..close];
        let port = s.get(close + 2..)?;
        (ip, port)
    } else {
        // IPv4: 1.2.3.4:12345
        let colon = s.rfind(':')?;
        (&s[..colon], &s[colon + 1..])
    };

    let ip = ip_str.parse().ok()?;
    let port = port_str.parse().ok()?;
    Some((ip, port))
}

/// Extract a `KEY=VALUE` field from a log line.
fn extract_field<'a>(line: &'a str, key: &str) -> Option<String> {
    let start = line.find(key)?;
    let remainder = &line[start + key.len()..];
    let end = remainder
        .find(' ')
        .unwrap_or(remainder.len());
    Some(remainder[..end].to_owned())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_input_returns_empty() {
        let entries = parse_iptables_log("", "TORIDE").unwrap();
        assert!(entries.is_empty());

        let entries = parse_conntrack_output("").unwrap();
        assert!(entries.is_empty());

        let entries = parse_ss_output("").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_addr_port_ipv4() {
        let (ip, port) = parse_addr_port("192.168.1.1:443").unwrap();
        assert_eq!(ip.to_string(), "192.168.1.1");
        assert_eq!(port, 443);
    }

    #[test]
    fn parse_addr_port_ipv6() {
        let (ip, port) = parse_addr_port("[::1]:8080").unwrap();
        assert_eq!(ip.to_string(), "::1");
        assert_eq!(port, 8080);
    }
}
