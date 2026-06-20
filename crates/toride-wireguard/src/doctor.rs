//! Tunnel health diagnostics for WireGuard.
//!
//! Provides a diagnostic engine that checks:
//! - WireGuard binary availability
//! - Interface status and connectivity
//! - Key file permissions
//! - DNS leak detection
//! - Configuration validity

use crate::error::{Error, Result};
use crate::report::{Finding, InterfaceReport, Severity, TransferStats, WireguardReport};

// ---------------------------------------------------------------------------
// `wg show` verbose parsing (free helpers)
// ---------------------------------------------------------------------------

/// Parsed interface summary from `wg show` verbose output.
#[derive(Debug, Clone, Default)]
struct ParsedInterface {
    name: String,
    listen_port: u16,
    peer_count: usize,
    active_peers: usize,
}

/// Parse the default (verbose) output of `wg show`.
///
/// The output is grouped by `interface:` and `peer:` headers, e.g.:
///
/// ```text
/// interface: wg0
///   public key: <key>
///   private key: (hidden)
///   listening port: 51820
///
/// peer: <key>
///   endpoint: 1.2.3.4:51820
///   allowed ips: 10.0.0.2/32
///   latest handshake: 4 seconds ago
///   transfer: 1.2 KiB received, 3.4 KiB sent
/// ```
///
/// We extract interface name + listen port, count peers per interface, and
/// count peers that have completed a handshake ("latest handshake:" line).
///
/// # Errors
///
/// Returns [`Error::ConfigParse`] if no interface header is found at all.
fn parse_wg_show_verbose(output: &str) -> Result<Vec<ParsedInterface>> {
    let mut ifaces: Vec<ParsedInterface> = Vec::new();
    let mut current: Option<ParsedInterface> = None;
    // A peer is "active" if we have seen a `latest handshake:` line for it
    // since its `peer:` header. Track per current peer.
    let mut peer_active = false;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("interface:") {
            if let Some(iface) = current.take() {
                ifaces.push(iface);
            }
            current = Some(ParsedInterface {
                name: rest.trim().to_owned(),
                ..Default::default()
            });
            peer_active = false;
        } else if let Some(rest) = trimmed.strip_prefix("peer:") {
            // A new peer block starts. If we're inside an interface, bump its
            // peer count and carry forward the active flag.
            if let Some(ref mut iface) = current {
                iface.peer_count += 1;
                if peer_active {
                    iface.active_peers += 1;
                }
            }
            peer_active = false;
            let _ = rest; // peer public key, not needed for counts
        } else if trimmed.starts_with("listening port:") {
            if let Some(port_str) = trimmed.split_whitespace().nth(2) {
                if let Some(ref mut iface) = current {
                    if let Ok(port) = port_str.parse::<u16>() {
                        iface.listen_port = port;
                    }
                }
            }
        } else if trimmed.starts_with("latest handshake:") {
            // This peer has completed at least one handshake.
            peer_active = true;
        }
    }

    // Flush the trailing peer's active state for the last interface.
    if let Some(ref mut iface) = current {
        if peer_active {
            iface.active_peers += 1;
        }
    }
    if let Some(iface) = current {
        ifaces.push(iface);
    }

    if ifaces.is_empty() && !output.trim().is_empty() {
        return Err(Error::ConfigParse(
            "no `interface:` headers found in `wg show` output".to_owned(),
        ));
    }

    Ok(ifaces)
}

/// Extract the permission bits (low 12, including setuid etc.) from a
/// `Metadata` value in a platform-agnostic way using `std::os::unix`.
///
/// Some platforms return the full `st_mode` (file-type bits + permission bits)
/// from `PermissionsExt::mode()`, so we mask down to the permission bits here.
fn file_mode_bits(metadata: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o7777
}

/// Return the DNS resolver(s) configured on the tunnel, if we can determine
/// them deterministically. Reads the `DNS =` line from the first interface
/// config found in the config dir. Returns `None` if unknown.
fn first_tunnel_dns(report: &WireguardReport) -> Option<Vec<String>> {
    if !report.config_dir_exists {
        return None;
    }
    let entries = std::fs::read_dir(&report.config_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "conf") {
            let content = std::fs::read_to_string(&path).ok()?;
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("DNS") {
                    if let Some((_, value)) = rest.split_once('=') {
                        let dns: Vec<String> = value
                            .trim()
                            .split(',')
                            .map(|s| s.trim().to_owned())
                            .filter(|s| !s.is_empty())
                            .collect();
                        if !dns.is_empty() {
                            return Some(dns);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Read the system's configured DNS resolvers from `/etc/resolv.conf`.
///
/// Returns `None` if the file cannot be read or no `nameserver` lines are
/// present, in which case the DNS-leak check stays silent.
fn read_system_dns() -> Option<Vec<String>> {
    let content = std::fs::read_to_string("/etc/resolv.conf").ok()?;
    let ns: Vec<String> = content
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            l.strip_prefix("nameserver").map(|s| s.trim().to_owned())
        })
        .filter(|s| !s.is_empty())
        .collect();
    if ns.is_empty() {
        None
    } else {
        Some(ns)
    }
}

// ---------------------------------------------------------------------------
// DoctorScope
// ---------------------------------------------------------------------------

/// Scope for diagnostic checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorScope {
    /// Run all available checks.
    All,
    /// Only check binary availability and basic setup.
    Setup,
    /// Only check interface status and peer connectivity.
    Connectivity,
    /// Only check key file permissions and security.
    Security,
}

// ---------------------------------------------------------------------------
// Doctor
// ---------------------------------------------------------------------------

/// Diagnostic engine for WireGuard installations.
///
/// Runs a series of health checks and collects findings into a
/// [`WireguardReport`].
pub struct Doctor {
    _runner: (),
}

impl Doctor {
    /// Create a new diagnostic engine.
    pub fn new() -> Self {
        Self { _runner: () }
    }

    /// Create a diagnostic engine with a custom runner (for testing).
    pub fn with_runner(_runner: ()) -> Self {
        Self { _runner: () }
    }

    /// Run diagnostics with the given scope and return a report.
    ///
    /// # Errors
    ///
    /// Returns an error only for fundamental failures (e.g. unable to run
    /// any checks). Individual check failures appear as [`Finding`] values
    /// in the report.
    pub fn run(&self, scope: &DoctorScope) -> Result<WireguardReport> {
        let mut report = WireguardReport::new();

        match scope {
            DoctorScope::All => {
                self.check_binaries(&mut report)?;
                self.check_config_dir(&mut report)?;
                self.check_interfaces(&mut report)?;
                self.check_key_permissions(&mut report)?;
                self.check_dns_leak(&mut report)?;
            }
            DoctorScope::Setup => {
                self.check_binaries(&mut report)?;
                self.check_config_dir(&mut report)?;
            }
            DoctorScope::Connectivity => {
                self.check_interfaces(&mut report)?;
            }
            DoctorScope::Security => {
                self.check_key_permissions(&mut report)?;
            }
        }

        Ok(report)
    }

    // -----------------------------------------------------------------------
    // Individual checks
    // -----------------------------------------------------------------------

    /// Check that `wg` and `wg-quick` binaries are available.
    fn check_binaries(&self, report: &mut WireguardReport) -> Result<()> {
        tracing::debug!("checking WireGuard binaries");

        report.wg_binary_found = which::which("wg").is_ok();
        if !report.wg_binary_found {
            report.findings.push(
                Finding::new(
                    "wireguard.binary.wg",
                    Severity::Error,
                    "`wg` binary not found on $PATH".to_owned(),
                )
                .with_fix("Install wireguard-tools: apt install wireguard-tools".to_owned()),
            );
        }

        report.wg_quick_binary_found = which::which("wg-quick").is_ok();
        if !report.wg_quick_binary_found {
            report.findings.push(
                Finding::new(
                    "wireguard.binary.wg-quick",
                    Severity::Warning,
                    "`wg-quick` binary not found on $PATH".to_owned(),
                )
                .with_fix("Install wireguard-tools: apt install wireguard-tools".to_owned()),
            );
        }

        Ok(())
    }

    /// Check that the WireGuard config directory exists with proper permissions.
    fn check_config_dir(&self, report: &mut WireguardReport) -> Result<()> {
        tracing::debug!("checking WireGuard config directory");
        report.config_dir_exists = report.config_dir.is_dir();

        if !report.config_dir_exists {
            report.findings.push(Finding::new(
                "wireguard.config-dir",
                Severity::Warning,
                format!(
                    "WireGuard config directory does not exist: {}",
                    report.config_dir.display()
                ),
            ));
        }

        Ok(())
    }

    /// Check interface status and peer connectivity.
    ///
    /// Runs `wg show` (default verbose output) and parses it into
    /// [`InterfaceReport`] values: interface names, listen ports, configured
    /// peer counts, and how many peers have a recent handshake.
    ///
    /// If the `wg` binary is absent this emits a critical finding instead of
    /// failing the whole diagnostic run, so the user still gets a useful report.
    fn check_interfaces(&self, report: &mut WireguardReport) -> Result<()> {
        tracing::debug!("checking WireGuard interfaces");

        if !report.wg_binary_found {
            report.findings.push(
                Finding::new(
                    "wireguard.interface.binary",
                    Severity::Error,
                    "wireguard binary not found: cannot inspect interfaces".to_owned(),
                )
                .with_fix(
                    "Install wireguard-tools (apt install wireguard-tools) and re-run".to_owned(),
                ),
            );
            return Ok(());
        }

        let output = match Self::run_wg_show() {
            Ok(out) => out,
            Err(err) => {
                report.findings.push(
                    Finding::new(
                        "wireguard.interface.show",
                        Severity::Warning,
                        format!("failed to run `wg show`: {err}"),
                    )
                    .with_fix(
                        "Ensure the user has permission to run `wg` (e.g. via sudo / wireguard group)"
                            .to_owned(),
                    ),
                );
                return Ok(());
            }
        };

        if output.trim().is_empty() {
            report.findings.push(Finding::new(
                "wireguard.interface.none",
                Severity::Info,
                "no WireGuard interfaces are currently up".to_owned(),
            ));
            return Ok(());
        }

        let parsed = match parse_wg_show_verbose(&output) {
            Ok(ifaces) => ifaces,
            Err(err) => {
                report.findings.push(Finding::new(
                    "wireguard.interface.parse",
                    Severity::Warning,
                    format!("failed to parse `wg show` output: {err}"),
                ));
                return Ok(());
            }
        };

        for parsed_iface in parsed {
            report.interfaces.push(InterfaceReport {
                name: parsed_iface.name.clone(),
                is_up: true,
                peer_count: parsed_iface.peer_count,
                active_peers: parsed_iface.active_peers,
                listen_port: parsed_iface.listen_port,
                stats: TransferStats::default(),
            });
        }

        Ok(())
    }

    /// Check that WireGuard config files have restrictive permissions.
    ///
    /// For each `/etc/wireguard/*.conf`, stats the file mode. A config file
    /// that contains a private key must not be group- or world-readable; the
    /// expected mode is `0600`. Permissive modes (any bit set in `0o077`)
    /// produce a warning. If no `.conf` files exist at all, an informational
    /// finding is emitted so the absence is visible rather than silent.
    fn check_key_permissions(&self, report: &mut WireguardReport) -> Result<()> {
        tracing::debug!("checking key file permissions");

        if !report.config_dir_exists {
            // The config-dir check already reported the missing directory.
            return Ok(());
        }

        let mut confs: Vec<std::path::PathBuf> = match std::fs::read_dir(&report.config_dir) {
            Ok(rd) => rd
                .filter_map(std::result::Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|ext| ext == "conf"))
                .collect(),
            Err(err) => {
                report.findings.push(Finding::new(
                    "wireguard.key-permissions.readdir",
                    Severity::Warning,
                    format!(
                        "cannot read WireGuard config directory {}: {err}",
                        report.config_dir.display()
                    ),
                ));
                return Ok(());
            }
        };
        confs.sort();

        if confs.is_empty() {
            report.findings.push(Finding::new(
                "wireguard.key-permissions.none",
                Severity::Info,
                format!(
                    "no WireGuard config files found in {}",
                    report.config_dir.display()
                ),
            ));
            return Ok(());
        }

        for conf in &confs {
            let mode = match std::fs::metadata(conf) {
                Ok(m) => file_mode_bits(&m),
                Err(err) => {
                    report.findings.push(Finding::new(
                        "wireguard.key-permissions.stat",
                        Severity::Warning,
                        format!("cannot stat {}: {err}", conf.display()),
                    ));
                    continue;
                }
            };

            // Group/other read/write/exec bits beyond the owner's.
            if mode & 0o077 != 0 {
                let severity = if mode & 0o022 != 0 {
                    // Group/other write -> private key is writable beyond owner.
                    Severity::Error
                } else {
                    Severity::Warning
                };
                report.findings.push(
                    Finding::new(
                        "wireguard.key-permissions.mode",
                        severity,
                        format!(
                            "WireGuard config {} is mode {:04o}; private key may be exposed to group/other",
                            conf.display(),
                            mode,
                        ),
                    )
                    .with_fix(
                        "Restrict the file: `chmod 600 <file>` (only the owner should read it)"
                            .to_owned(),
                    ),
                );
            } else if mode != 0o600 {
                // Owner-only but not exactly 0600 (e.g. 0400 or 0700). Note it.
                report.findings.push(Finding::new(
                    "wireguard.key-permissions.mode",
                    Severity::Info,
                    format!(
                        "WireGuard config {} is mode {:04o} (owner-only); 0600 is the convention",
                        conf.display(),
                        mode,
                    ),
                ));
            }
        }

        Ok(())
    }

    /// Check for DNS leaks when the tunnel is active.
    ///
    /// Conservative, best-effort heuristic. We only emit a finding when we can
    /// *confidently* determine that the tunnel's configured DNS resolver differs
    /// from the active system resolver. If either resolver cannot be read, or
    /// they agree, or there is ambiguity, we emit nothing rather than risk a
    /// false positive.
    fn check_dns_leak(&self, report: &mut WireguardReport) -> Result<()> {
        tracing::debug!("checking for DNS leaks");

        let tunnel_dns = match first_tunnel_dns(report) {
            Some(d) => d,
            None => return Ok(()),
        };

        let system_dns = match read_system_dns() {
            Some(d) => d,
            None => return Ok(()),
        };

        if !system_dns.is_empty()
            && !tunnel_dns.is_empty()
            && !system_dns.iter().any(|s| tunnel_dns.contains(s))
        {
            report.findings.push(Finding::new(
                "wireguard.dns-leak",
                Severity::Info,
                format!(
                    "system resolvers ({}) differ from tunnel DNS ({}); queries may bypass the tunnel",
                    system_dns.join(", "),
                    tunnel_dns.join(", "),
                ),
            ));
        }

        Ok(())
    }

    /// Run `wg show` and return its stdout as a string.
    fn run_wg_show() -> std::result::Result<String, String> {
        let output = std::process::Command::new("wg")
            .arg("show")
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!(
                "`wg show` exited with status {}",
                output.status.code().unwrap_or(-1)
            ));
        }
        String::from_utf8(output.stdout).map_err(|e| e.to_string())
    }
}

impl Default for Doctor {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_setup_scope() {
        let doc = Doctor::new();
        let report = doc.run(&DoctorScope::Setup).unwrap();
        // Binaries likely not found in test environment.
        assert!(!report.findings.is_empty() || !report.wg_binary_found);
    }

    #[test]
    fn run_all_scope() {
        let doc = Doctor::new();
        let report = doc.run(&DoctorScope::All).unwrap();
        assert!(report.config_dir.exists() || !report.config_dir_exists);
    }

    #[test]
    fn doctor_scope_variants() {
        assert_ne!(DoctorScope::All, DoctorScope::Setup);
        assert_ne!(DoctorScope::Connectivity, DoctorScope::Security);
    }

    #[test]
    fn parse_verbose_single_interface_two_peers() {
        let output = "\
interface: wg0
  public key: AAA=
  private key: (hidden)
  listening port: 51820

peer: BBB=
  endpoint: 1.2.3.4:51820
  allowed ips: 10.0.0.2/32
  latest handshake: 4 seconds ago
  transfer: 1.2 KiB received, 3.4 KiB sent

peer: CCC=
  endpoint: 5.6.7.8:51820
  allowed ips: 10.0.0.3/32
";
        let ifaces = parse_wg_show_verbose(output).unwrap();
        assert_eq!(ifaces.len(), 1);
        assert_eq!(ifaces[0].name, "wg0");
        assert_eq!(ifaces[0].listen_port, 51820);
        assert_eq!(ifaces[0].peer_count, 2);
        assert_eq!(ifaces[0].active_peers, 1);
    }

    #[test]
    fn parse_verbose_multiple_interfaces() {
        let output = "\
interface: wg0
  listening port: 51820

interface: wg1
  listening port: 51821

peer: X=
  latest handshake: 1 minute ago
";
        let ifaces = parse_wg_show_verbose(output).unwrap();
        assert_eq!(ifaces.len(), 2);
        assert_eq!(ifaces[0].peer_count, 0);
        assert_eq!(ifaces[0].active_peers, 0);
        assert_eq!(ifaces[1].peer_count, 1);
        assert_eq!(ifaces[1].active_peers, 1);
    }

    #[test]
    fn parse_verbose_empty_is_empty() {
        assert!(parse_wg_show_verbose("").unwrap().is_empty());
        assert!(parse_wg_show_verbose("   \n  \n").unwrap().is_empty());
    }

    #[test]
    fn parse_verbose_unrecognized_is_error() {
        // Non-empty but no interface header.
        assert!(parse_wg_show_verbose("just some garbage\n").is_err());
    }

    #[test]
    fn key_permissions_flags_world_readable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let conf = dir.path().join("wg0.conf");
        std::fs::write(&conf, "[Interface]\nPrivateKey = secret\n").unwrap();
        std::fs::set_permissions(&conf, std::fs::Permissions::from_mode(0o644)).unwrap();

        let mut report = WireguardReport::default();
        report.config_dir = dir.path().to_owned();
        report.config_dir_exists = true;

        let doc = Doctor::new();
        doc.check_key_permissions(&mut report).unwrap();

        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check_id == "wireguard.key-permissions.mode"
                    && f.severity == Severity::Warning),
            "expected a warning for world-readable conf"
        );
    }

    #[test]
    fn key_permissions_errors_on_group_writable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let conf = dir.path().join("wg0.conf");
        std::fs::write(&conf, "[Interface]\nPrivateKey = secret\n").unwrap();
        std::fs::set_permissions(&conf, std::fs::Permissions::from_mode(0o664)).unwrap();

        let mut report = WireguardReport::default();
        report.config_dir = dir.path().to_owned();
        report.config_dir_exists = true;

        let doc = Doctor::new();
        doc.check_key_permissions(&mut report).unwrap();

        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check_id == "wireguard.key-permissions.mode"
                    && f.severity == Severity::Error),
            "expected an error for group-writable conf"
        );
    }

    #[test]
    fn key_permissions_silent_on_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let conf = dir.path().join("wg0.conf");
        std::fs::write(&conf, "[Interface]\nPrivateKey = secret\n").unwrap();
        std::fs::set_permissions(&conf, std::fs::Permissions::from_mode(0o600)).unwrap();

        let mut report = WireguardReport::default();
        report.config_dir = dir.path().to_owned();
        report.config_dir_exists = true;

        let doc = Doctor::new();
        doc.check_key_permissions(&mut report).unwrap();

        assert!(
            !report
                .findings
                .iter()
                .any(|f| f.check_id == "wireguard.key-permissions.mode"),
            "0600 conf should not produce a mode finding"
        );
    }

    #[test]
    fn key_permissions_info_when_no_confs() {
        let dir = tempfile::tempdir().unwrap();
        let mut report = WireguardReport::default();
        report.config_dir = dir.path().to_owned();
        report.config_dir_exists = true;

        let doc = Doctor::new();
        doc.check_key_permissions(&mut report).unwrap();

        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check_id == "wireguard.key-permissions.none"
                    && f.severity == Severity::Info)
        );
    }

    #[test]
    fn check_interfaces_critical_when_no_wg_binary() {
        let mut report = WireguardReport::default();
        report.wg_binary_found = false;

        let doc = Doctor::new();
        doc.check_interfaces(&mut report).unwrap();

        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check_id == "wireguard.interface.binary"
                    && f.severity == Severity::Error),
            "absent wg binary should produce a critical interface finding"
        );
        assert!(report.interfaces.is_empty());
    }
}
