//! Doctor module — structured diagnostic checks for UFW.
//!
//! Returns `Vec<Finding>` rather than just text. Each finding has an ID,
//! severity, title, detail, and optional fix suggestion.

use crate::error::Result;
use crate::spec::{DoctorScope, Finding, Severity};
use crate::Ufw;

/// Run doctor checks and return findings.
pub fn doctor(ufw: &Ufw, scope: DoctorScope) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();

    match scope {
        DoctorScope::All => {
            findings.extend(check_binaries(ufw));
            findings.extend(check_service(ufw));
            findings.extend(check_config(ufw));
            findings.extend(check_policy(ufw));
            findings.extend(check_rules(ufw));
            findings.extend(check_ssh(ufw));
            findings.extend(check_ipv6(ufw));
            findings.extend(check_logging(ufw));
            findings.extend(check_app_profiles(ufw));
            findings.extend(check_permissions(ufw));
        }
        DoctorScope::Binaries => findings.extend(check_binaries(ufw)),
        DoctorScope::Service => findings.extend(check_service(ufw)),
        DoctorScope::Policy => findings.extend(check_policy(ufw)),
        DoctorScope::Rules => findings.extend(check_rules(ufw)),
        DoctorScope::Ssh => findings.extend(check_ssh(ufw)),
        DoctorScope::Ipv6 => findings.extend(check_ipv6(ufw)),
        DoctorScope::Logging => findings.extend(check_logging(ufw)),
        DoctorScope::AppProfiles => findings.extend(check_app_profiles(ufw)),
        DoctorScope::Permissions => findings.extend(check_permissions(ufw)),
    }

    Ok(findings)
}

/// Check that required binaries exist.
#[allow(clippy::too_many_lines)]
fn check_binaries(ufw: &Ufw) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Check ufw binary
    if ufw.find_ufw().is_ok() {
        findings.push(Finding {
            id: "bin:ufw:exists",
            severity: Severity::Ok,
            title: "UFW binary found".into(),
            detail: "The ufw binary is available on this system.".into(),
            fix: None,
        });
    } else {
        findings.push(Finding {
            id: "bin:ufw:missing",
            severity: Severity::Critical,
            title: "UFW binary not found".into(),
            detail: "The ufw binary is not installed or not in PATH.".into(),
            fix: Some("Install ufw: sudo apt install ufw (Debian/Ubuntu) or sudo pacman -S ufw (Arch)".into()),
        });
        return findings; // No point checking further
    }

    // Check ufw version
    match ufw.version() {
        Ok(ver) => findings.push(Finding {
            id: "bin:ufw:version",
            severity: Severity::Ok,
            title: "UFW version detected".into(),
            detail: format!("UFW version: {ver}"),
            fix: None,
        }),
        Err(_) => findings.push(Finding {
            id: "bin:ufw:version-fail",
            severity: Severity::Warning,
            title: "Could not read UFW version".into(),
            detail: "ufw --version returned an unexpected response.".into(),
            fix: None,
        }),
    }

    // Check iptables
    let runner = ufw.runner();
    if runner.binary_exists("iptables") {
        findings.push(Finding {
            id: "bin:iptables:exists",
            severity: Severity::Ok,
            title: "iptables binary found".into(),
            detail: "The iptables binary is available on this system.".into(),
            fix: None,
        });
    } else {
        findings.push(Finding {
            id: "bin:iptables:missing",
            severity: Severity::Critical,
            title: "iptables binary not found".into(),
            detail: "The iptables binary is not installed or not in PATH. UFW depends on iptables.".into(),
            fix: Some("Install iptables: sudo apt install iptables (Debian/Ubuntu)".into()),
        });
    }

    // Check ip6tables
    if runner.binary_exists("ip6tables") {
        findings.push(Finding {
            id: "bin:ip6tables:exists",
            severity: Severity::Ok,
            title: "ip6tables binary found".into(),
            detail: "The ip6tables binary is available on this system.".into(),
            fix: None,
        });
    } else {
        findings.push(Finding {
            id: "bin:ip6tables:missing",
            severity: Severity::Warning,
            title: "ip6tables binary not found".into(),
            detail: "The ip6tables binary is not installed. IPv6 firewall rules may not function.".into(),
            fix: Some("Install iptables for IPv6: sudo apt install iptables (Debian/Ubuntu)".into()),
        });
    }

    // Check nft (info level — not critical for UFW)
    if runner.binary_exists("nft") {
        findings.push(Finding {
            id: "bin:nft:exists",
            severity: Severity::Info,
            title: "nftables binary found".into(),
            detail: "The nft binary is available. Not required for UFW but good for modern systems.".into(),
            fix: None,
        });
    } else {
        findings.push(Finding {
            id: "bin:nft:missing",
            severity: Severity::Info,
            title: "nftables binary not found".into(),
            detail: "The nft binary is not installed. Not required for UFW but useful on modern systems.".into(),
            fix: None,
        });
    }

    // Check systemctl
    if runner.binary_exists("systemctl") {
        findings.push(Finding {
            id: "bin:systemctl:exists",
            severity: Severity::Ok,
            title: "systemctl binary found".into(),
            detail: "The systemctl binary is available on this system.".into(),
            fix: None,
        });
    } else {
        findings.push(Finding {
            id: "bin:systemctl:missing",
            severity: Severity::Warning,
            title: "systemctl binary not found".into(),
            detail: "The systemctl binary is not installed. Service management checks will be limited.".into(),
            fix: None,
        });
    }

    findings
}

/// Check UFW service status.
fn check_service(ufw: &Ufw) -> Vec<Finding> {
    let mut findings = Vec::new();

    let status_result = ufw.status();

    match status_result {
        Ok(status) => {
            let ufw_active = status.active;

            if ufw_active {
                findings.push(Finding {
                    id: "svc:ufw:active",
                    severity: Severity::Ok,
                    title: "UFW is active".into(),
                    detail: "The UFW firewall is currently active.".into(),
                    fix: None,
                });
            } else {
                findings.push(Finding {
                    id: "svc:ufw:inactive",
                    severity: Severity::Warning,
                    title: "UFW is inactive".into(),
                    detail: "The UFW firewall is not currently active.".into(),
                    fix: Some("Enable UFW with: ufw enable (after ensuring SSH is allowed)".into()),
                });
            }

            // Check consistency with systemctl
            let runner = ufw.runner();
            if runner.binary_exists("systemctl") {
                let svc_active = crate::service::is_active(runner).ok();
                if let Some(svc) = svc_active {
                    if ufw_active && !svc {
                        findings.push(Finding {
                            id: "svc:ufw:inconsistent",
                            severity: Severity::Warning,
                            title: "UFW status inconsistency detected".into(),
                            detail: "UFW reports active but systemctl reports the service is not active.".into(),
                            fix: Some("Restart the UFW service: sudo systemctl restart ufw".into()),
                        });
                    } else if !ufw_active && svc {
                        findings.push(Finding {
                            id: "svc:systemd:inconsistent",
                            severity: Severity::Warning,
                            title: "UFW status inconsistency detected".into(),
                            detail: "UFW reports inactive but systemctl reports the service is active.".into(),
                            fix: Some("Check UFW status: sudo ufw status verbose".into()),
                        });
                    }
                }

                // Check if service is enabled (boot-time start)
                let svc_enabled = crate::service::is_enabled(runner).ok();
                if let Some(enabled) = svc_enabled {
                    if ufw_active && !enabled {
                        findings.push(Finding {
                            id: "svc:ufw:not-enabled",
                            severity: Severity::Warning,
                            title: "UFW service not enabled at boot".into(),
                            detail: "UFW is active but the service is not enabled. It may not start after reboot.".into(),
                            fix: Some("Enable the service: sudo systemctl enable ufw".into()),
                        });
                    }
                }
            }

            // Check default incoming policy and warn if allow
            if let Some(policy) = &status.default_incoming {
                if matches!(policy, crate::spec::Policy::Allow) {
                    findings.push(Finding {
                        id: "svc:pol:incoming-allow",
                        severity: Severity::Warning,
                        title: "Default incoming policy is ALLOW while checking service".into(),
                        detail: "The default incoming policy is set to allow, which is too permissive for most servers.".into(),
                        fix: Some("Set default incoming to deny: sudo ufw default deny incoming".into()),
                    });
                }
            }
        }
        Err(e) => findings.push(Finding {
            id: "svc:ufw:status-fail",
            severity: Severity::Error,
            title: "Could not read UFW status".into(),
            detail: format!("Failed to read UFW status: {e}"),
            fix: Some("Check that ufw is installed and you have sufficient permissions.".into()),
        }),
    }

    findings
}

/// Check UFW configuration files.
#[allow(clippy::too_many_lines)]
fn check_config(ufw: &Ufw) -> Vec<Finding> {
    let mut findings = Vec::new();

    let config_files = [
        "/etc/default/ufw",
        "/etc/ufw/ufw.conf",
        "/etc/ufw/sysctl.conf",
        "/etc/ufw/before.rules",
        "/etc/ufw/after.rules",
    ];

    for path in &config_files {
        if std::path::Path::new(path).exists() {
            findings.push(Finding {
                id: Box::leak(format!("cfg:{}:exists", path.replace('/', ":")).into_boxed_str()),
                severity: Severity::Ok,
                title: format!("{path} exists"),
                detail: format!("Configuration file {path} is present."),
                fix: None,
            });
        } else {
            findings.push(Finding {
                id: Box::leak(format!("cfg:{}:missing", path.replace('/', ":")).into_boxed_str()),
                severity: Severity::Warning,
                title: format!("{path} missing"),
                detail: format!("Configuration file {path} does not exist."),
                fix: Some(format!("Reinstall ufw or create {path} manually.")),
            });
        }
    }

    // Parse /etc/default/ufw using the config module for structured checks
    let default_ufw_config = read_default_ufw_config();
    let ipv6_enabled = default_ufw_config.ipv6.unwrap_or(false);

    // Parse /etc/ufw/ufw.conf using the config module
    let ufw_conf = read_ufw_conf();

    // Check ENABLED consistency between /etc/default/ufw and /etc/ufw/ufw.conf
    if let (Some(default_enabled), Some(conf_enabled)) =
        (default_ufw_config.enabled, ufw_conf.enabled)
    {
        if default_enabled != conf_enabled {
            findings.push(Finding {
                id: "cfg:enabled:inconsistent",
                severity: Severity::Warning,
                title: "ENABLED value inconsistent between config files".into(),
                detail: format!(
                    "/etc/default/ufw has ENABLED={}, but /etc/ufw/ufw.conf has ENABLED={}. \
                     These should agree.",
                    if default_enabled { "yes" } else { "no" },
                    if conf_enabled { "yes" } else { "no" }
                ),
                fix: Some("Align the ENABLED values in /etc/default/ufw and /etc/ufw/ufw.conf.".into()),
            });
        }
    }

    // Check that LOGLEVEL in ufw.conf is a valid value
    if let Some(ref level) = ufw_conf.loglevel {
        let valid_levels = ["off", "low", "medium", "high", "full", "on"];
        if !valid_levels.contains(&level.to_lowercase().as_str()) {
            findings.push(Finding {
                id: "cfg:loglevel:invalid",
                severity: Severity::Warning,
                title: "Invalid LOGLEVEL in ufw.conf".into(),
                detail: format!(
                    "LOGLEVEL is set to '{level}' in /etc/ufw/ufw.conf, which is not a recognized value. \
                     Valid values: off, low, medium, high, full, on."
                ),
                fix: Some("Set LOGLEVEL to a valid value in /etc/ufw/ufw.conf.".into()),
            });
        }
    }

    // When IPv6 is enabled, check IPv6-specific files exist
    if ipv6_enabled {
        let ipv6_files = ["/etc/ufw/before6.rules", "/etc/ufw/after6.rules"];
        for path in &ipv6_files {
            if std::path::Path::new(path).exists() {
                findings.push(Finding {
                    id: Box::leak(format!("cfg:{}:exists", path.replace('/', ":")).into_boxed_str()),
                    severity: Severity::Ok,
                    title: format!("{path} exists"),
                    detail: format!("IPv6 framework file {path} is present."),
                    fix: None,
                });
            } else {
                findings.push(Finding {
                    id: Box::leak(format!("cfg:{}:missing", path.replace('/', ":")).into_boxed_str()),
                    severity: Severity::Warning,
                    title: format!("{path} missing"),
                    detail: format!("IPv6 is enabled but {path} does not exist."),
                    fix: Some(format!("Reinstall ufw or create {path} to enable IPv6 rules.")),
                });
            }
        }
    }

    // Check that generated app profiles have managed header
    let app_dir = std::path::Path::new("/etc/ufw/applications.d");
    if app_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(app_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        // Check if this looks like a generated profile (has [Section] header)
                        let has_section = content.lines().any(|l| l.trim().starts_with('[') && l.trim().ends_with(']'));
                        if has_section && !content.contains("Managed by ufw-kit") {
                            findings.push(Finding {
                                id: Box::leak(format!("cfg:app:{name}:no-managed-header").into_boxed_str()),
                                severity: Severity::Info,
                                title: format!("App profile {name} lacks managed header"),
                                detail: format!("The app profile at {} has a section header but no 'Managed by ufw-kit' header.", path.display()),
                                fix: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // Silence unused parameter warning — ufw is used for future extensibility
    let _ = ufw;

    findings
}

/// Check default policies.
fn check_policy(ufw: &Ufw) -> Vec<Finding> {
    let mut findings = Vec::new();

    match ufw.status_verbose() {
        Ok(status) => {
            // Check incoming policy
            if let Some(policy) = &status.default_incoming {
                match policy {
                    crate::spec::Policy::Allow => {
                        findings.push(Finding {
                            id: "pol:incoming:allow",
                            severity: Severity::Warning,
                            title: "Default incoming policy is ALLOW".into(),
                            detail: "Servers should typically have incoming DENY or REJECT as default.".into(),
                            fix: Some("Set default incoming to deny: ufw default deny incoming".into()),
                        });
                    }
                    crate::spec::Policy::Deny | crate::spec::Policy::Reject => {
                        findings.push(Finding {
                            id: "pol:incoming:deny",
                            severity: Severity::Ok,
                            title: "Default incoming policy is secure".into(),
                            detail: format!("Default incoming policy: {policy}"),
                            fix: None,
                        });
                    }
                }
            }

            // Check outgoing policy
            if let Some(policy) = &status.default_outgoing {
                match policy {
                    crate::spec::Policy::Deny | crate::spec::Policy::Reject => {
                        findings.push(Finding {
                            id: "pol:outgoing:deny",
                            severity: Severity::Warning,
                            title: "Default outgoing policy is restrictive".into(),
                            detail: "Outgoing deny can break DNS, NTP, and package updates.".into(),
                            fix: Some("Ensure DNS (53/udp), NTP (123/udp), and HTTPS (443/tcp) are allowed.".into()),
                        });

                        // When outgoing is deny/reject, check if DNS/NTP/HTTPS rules exist
                        check_essential_outgoing_rules(ufw, &mut findings);
                    }
                    crate::spec::Policy::Allow => {
                        findings.push(Finding {
                            id: "pol:outgoing:allow",
                            severity: Severity::Ok,
                            title: "Default outgoing policy is allow".into(),
                            detail: "Default outgoing policy: allow".into(),
                            fix: None,
                        });
                    }
                }
            }

            // Check routed/forwarded policy
            if let Some(policy) = &status.default_routed {
                match policy {
                    crate::spec::Policy::Allow => {
                        findings.push(Finding {
                            id: "pol:routed:allow",
                            severity: Severity::Warning,
                            title: "Default routed policy is ALLOW".into(),
                            detail: "The default routed/forwarded policy is allow. This can expose internal networks if forwarding is enabled.".into(),
                            fix: Some("Set default routed to deny: ufw default deny routed".into()),
                        });
                    }
                    crate::spec::Policy::Deny | crate::spec::Policy::Reject => {
                        findings.push(Finding {
                            id: "pol:routed:deny",
                            severity: Severity::Ok,
                            title: "Default routed policy is secure".into(),
                            detail: format!("Default routed/forwarded policy: {policy}"),
                            fix: None,
                        });
                    }
                }
            } else {
                // Routed policy not explicitly set — report as info
                findings.push(Finding {
                    id: "pol:routed:unset",
                    severity: Severity::Info,
                    title: "Default routed policy not set".into(),
                    detail: "The default routed/forwarded policy is not explicitly configured.".into(),
                    fix: None,
                });
            }
        }
        Err(e) => findings.push(Finding {
            id: "pol:status-fail",
            severity: Severity::Error,
            title: "Could not check policies".into(),
            detail: format!("Failed to read verbose status: {e}"),
            fix: None,
        }),
    }

    findings
}

/// Check that essential outgoing rules (DNS, NTP, HTTPS) exist when outgoing policy is deny.
fn check_essential_outgoing_rules(ufw: &Ufw, findings: &mut Vec<Finding>) {
    if let Ok(status) = ufw.status() {
        let rules_text: String = status.rules.iter().map(|r| r.raw.to_lowercase()).collect::<Vec<_>>().join("\n");

        // DNS (53)
        let has_dns = rules_text.contains("53")
            && (rules_text.contains("udp") || rules_text.contains("53/udp") || rules_text.contains("domain"));
        if !has_dns {
            findings.push(Finding {
                id: "pol:outgoing:no-dns",
                severity: Severity::Warning,
                title: "No DNS rule with restrictive outgoing policy".into(),
                detail: "Outgoing traffic is denied but no DNS (53/udp) allow rule was found. DNS resolution will fail.".into(),
                fix: Some("Add a DNS allow rule: ufw allow out 53/udp".into()),
            });
        }

        // NTP (123)
        let has_ntp = rules_text.contains("123")
            && (rules_text.contains("udp") || rules_text.contains("123/udp") || rules_text.contains("ntp"));
        if !has_ntp {
            findings.push(Finding {
                id: "pol:outgoing:no-ntp",
                severity: Severity::Warning,
                title: "No NTP rule with restrictive outgoing policy".into(),
                detail: "Outgoing traffic is denied but no NTP (123/udp) allow rule was found. Clock sync will fail.".into(),
                fix: Some("Add an NTP allow rule: ufw allow out 123/udp".into()),
            });
        }

        // HTTPS (443)
        let has_https = rules_text.contains("443")
            && (rules_text.contains("tcp") || rules_text.contains("443/tcp") || rules_text.contains("https"));
        if !has_https {
            findings.push(Finding {
                id: "pol:outgoing:no-https",
                severity: Severity::Warning,
                title: "No HTTPS rule with restrictive outgoing policy".into(),
                detail: "Outgoing traffic is denied but no HTTPS (443/tcp) allow rule was found. Package updates and API calls will fail.".into(),
                fix: Some("Add an HTTPS allow rule: ufw allow out 443/tcp".into()),
            });
        }
    }
}

/// Check rules for safety issues.
fn check_rules(ufw: &Ufw) -> Vec<Finding> {
    let mut findings = Vec::new();

    match ufw.status() {
        Ok(status) => {
            // Check for dangerous ports
            for rule in &status.rules {
                let raw = rule.raw.to_lowercase();
                for &(port, name) in crate::net::DANGEROUS_PORTS {
                    if raw.contains(&port.to_string()) && raw.contains("allow") {
                        findings.push(Finding {
                            id: Box::leak(
                                format!("rule:dangerous:{port}")
                                    .into_boxed_str(),
                            ),
                            severity: Severity::Warning,
                            title: format!("Port {port} ({name}) is exposed"),
                            detail: format!(
                                "Rule exposes port {port} ({name}): {}",
                                rule.raw
                            ),
                            fix: Some(format!(
                                "Consider restricting access to port {port} from trusted IPs only."
                            )),
                        });
                    }
                }
            }

            // Check for broad allow rules
            for rule in &status.rules {
                let raw = rule.raw.to_lowercase();
                if raw.contains("allow") && raw.contains("anywhere") {
                    findings.push(Finding {
                        id: "rule:broad:allow-anywhere",
                        severity: Severity::Info,
                        title: "Broad allow rule detected".into(),
                        detail: format!("Rule allows traffic from anywhere: {}", rule.raw),
                        fix: Some("Consider restricting source to specific IPs or CIDR ranges.".into()),
                    });
                }
            }

            // Detect duplicate rules (same raw text appearing twice)
            let mut seen_raw: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
            for rule in &status.rules {
                let count = seen_raw.entry(&rule.raw).or_insert(0);
                *count += 1;
                if *count == 2 {
                    // Only report on the second occurrence
                    findings.push(Finding {
                        id: "rule:duplicate",
                        severity: Severity::Warning,
                        title: "Duplicate rule detected".into(),
                        detail: format!("Rule appears more than once: {}", rule.raw),
                        fix: Some("Remove the duplicate rule: ufw delete 'the rule'".into()),
                    });
                }
            }

            // Check for rules without comments
            for rule in &status.rules {
                if rule.comment.is_none() || rule.comment.as_deref().is_none_or(|c| c.trim().is_empty()) {
                    findings.push(Finding {
                        id: "rule:no-comment",
                        severity: Severity::Info,
                        title: "Rule without comment".into(),
                        detail: format!("Rule has no comment annotation: {}", rule.raw),
                        fix: Some("Consider adding a comment: ufw ... comment 'description'".into()),
                    });
                }
            }

            // Check managed comment prefix validation
            for rule in &status.rules {
                if let Some(comment) = &rule.comment {
                    if comment.contains("managed:") {
                        // Validate the managed prefix format
                        let parts: Vec<&str> = comment.splitn(2, ':').collect();
                        if parts.len() < 2 || parts[1].trim().is_empty() {
                            findings.push(Finding {
                                id: "rule:managed:bad-prefix",
                                severity: Severity::Info,
                                title: "Managed rule has empty prefix label".into(),
                                detail: format!("Rule comment '{comment}' has a managed: prefix but no label."),
                                fix: Some("Update the rule comment with a descriptive label after 'managed:'.".into()),
                            });
                        }
                    }
                }
            }
        }
        Err(e) => findings.push(Finding {
            id: "rule:status-fail",
            severity: Severity::Error,
            title: "Could not check rules".into(),
            detail: format!("Failed to read status: {e}"),
            fix: None,
        }),
    }

    findings
}

/// Check SSH access safety.
fn check_ssh(ufw: &Ufw) -> Vec<Finding> {
    let mut findings = Vec::new();

    if let Ok(status) = ufw.status() {
        let ssh_rules: Vec<_> = status.rules.iter().filter(|rule| {
            let raw = rule.raw.to_lowercase();
            raw.contains("22") || raw.contains("ssh")
        }).collect();

        let has_ssh_rule = !ssh_rules.is_empty();

        if status.active {
            if has_ssh_rule {
                findings.push(Finding {
                    id: "ssh:allowed",
                    severity: Severity::Ok,
                    title: "SSH access is allowed".into(),
                    detail: "An SSH allow rule exists in the firewall.".into(),
                    fix: None,
                });

                // Check if SSH rule uses "limit" action (good practice)
                let uses_limit = ssh_rules.iter().any(|rule| {
                    let raw = rule.raw.to_lowercase();
                    raw.contains("limit")
                });
                if uses_limit {
                    findings.push(Finding {
                        id: "ssh:limit",
                        severity: Severity::Ok,
                        title: "SSH rule uses rate limiting".into(),
                        detail: "The SSH rule uses the 'limit' action, which provides brute-force protection.".into(),
                        fix: None,
                    });
                }

                // Check if SSH rule is scoped to a specific interface (info)
                let scoped_to_interface = ssh_rules.iter().any(|rule| {
                    let raw = rule.raw.to_lowercase();
                    raw.contains("on ") || raw.contains("in on ") || raw.contains("out on ")
                });
                if scoped_to_interface {
                    findings.push(Finding {
                        id: "ssh:interface-scoped",
                        severity: Severity::Info,
                        title: "SSH rule is interface-scoped".into(),
                        detail: "At least one SSH rule is scoped to a specific network interface.".into(),
                        fix: None,
                    });
                }

                // Check if SSH rule allows from anywhere (warning)
                let allows_anywhere = ssh_rules.iter().any(|rule| {
                    let raw = rule.raw.to_lowercase();
                    (raw.contains("anywhere") || raw.contains("0.0.0.0") || raw.contains("::/0"))
                        && raw.contains("allow")
                });
                if allows_anywhere {
                    findings.push(Finding {
                        id: "ssh:allow-anywhere",
                        severity: Severity::Warning,
                        title: "SSH allows connections from anywhere".into(),
                        detail: "An SSH rule allows connections from any source address. This exposes SSH to brute-force attacks.".into(),
                        fix: Some("Restrict SSH to trusted IPs: ufw allow from <trusted-ip> to any port 22 proto tcp".into()),
                    });
                }
            } else {
                findings.push(Finding {
                    id: "ssh:no-rule",
                    severity: Severity::Critical,
                    title: "No SSH allow rule found".into(),
                    detail: "UFW is active but no SSH allow rule exists. This may lock you out.".into(),
                    fix: Some("Add SSH allow rule: ufw allow 22/tcp".into()),
                });
            }
        }
    }

    findings
}

/// Check IPv6 configuration.
fn check_ipv6(ufw: &Ufw) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Read /etc/default/ufw using the config module instead of raw fs::read
    let ipv6_enabled = read_ipv6_enabled_from_config();

    if ipv6_enabled {
        findings.push(Finding {
            id: "ipv6:enabled",
            severity: Severity::Ok,
            title: "IPv6 is enabled in UFW".into(),
            detail: "IPV6=yes is set in /etc/default/ufw.".into(),
            fix: None,
        });

        // Check for IPv6 rules
        if let Ok(status) = ufw.status() {
            let ipv6_rules: Vec<_> = status.rules.iter().filter(|r| r.ipv6).collect();
            if ipv6_rules.is_empty() {
                findings.push(Finding {
                    id: "ipv6:no-rules",
                    severity: Severity::Info,
                    title: "No IPv6-specific rules".into(),
                    detail: "IPv6 is enabled but no IPv6-specific rules were found.".into(),
                    fix: Some("Consider adding IPv6 rules for dual-stack coverage.".into()),
                });
            } else {
                findings.push(Finding {
                    id: "ipv6:rules-found",
                    severity: Severity::Ok,
                    title: "IPv6 rules present".into(),
                    detail: format!("Found {} IPv6-specific rule(s).", ipv6_rules.len()),
                    fix: None,
                });
            }
        }

        // Check IPv6 default policies match IPv4
        if let Ok(status) = ufw.status_verbose() {
            let v4_incoming = status.default_incoming;
            // Read the config file to get the configured IPv6 policy
            let config = read_default_ufw_config();

            if let (Some(v4_pol), Some(ipv6_pol_str)) = (&v4_incoming, &config.default_input_policy) {
                let v4_str = v4_pol.to_string().to_lowercase();
                if v4_str != ipv6_pol_str.to_lowercase() {
                    findings.push(Finding {
                        id: "ipv6:policy-mismatch",
                        severity: Severity::Warning,
                        title: "IPv6 input policy differs from IPv4".into(),
                        detail: format!("IPv4 incoming policy is {v4_str} but IPv6 (DEFAULT_INPUT_POLICY) is set to {ipv6_pol_str}."),
                        fix: Some("Align IPv6 policy with IPv4 in /etc/default/ufw.".into()),
                    });
                }
            }
        }
    } else {
        findings.push(Finding {
            id: "ipv6:disabled",
            severity: Severity::Info,
            title: "IPv6 is disabled in UFW".into(),
            detail: "IPV6 is not set to 'yes' in /etc/default/ufw.".into(),
            fix: Some("Enable IPv6 if your VPS has a public IPv6 address.".into()),
        });
    }

    findings
}

/// Check logging configuration.
fn check_logging(ufw: &Ufw) -> Vec<Finding> {
    let mut findings = Vec::new();

    if let Ok(status) = ufw.status_verbose() {
        if let Some(level) = &status.logging_level {
            match level {
                crate::spec::LoggingLevel::Off => {
                    findings.push(Finding {
                        id: "log:off",
                        severity: Severity::Warning,
                        title: "Logging is disabled".into(),
                        detail: "UFW logging is currently off. This makes troubleshooting difficult.".into(),
                        fix: Some("Enable logging: ufw logging on".into()),
                    });
                }
                crate::spec::LoggingLevel::High | crate::spec::LoggingLevel::Full => {
                    findings.push(Finding {
                        id: "log:high",
                        severity: Severity::Warning,
                        title: "Logging level is high/full".into(),
                        detail: format!("Logging level: {level}. This can generate significant disk I/O."),
                        fix: Some("Consider using 'low' or 'medium' logging, or per-rule logging instead.".into()),
                    });
                }
                crate::spec::LoggingLevel::Low
                | crate::spec::LoggingLevel::Medium
                | crate::spec::LoggingLevel::On => {
                    findings.push(Finding {
                        id: "log:ok",
                        severity: Severity::Ok,
                        title: "Logging level is reasonable".into(),
                        detail: format!("Logging level: {level}"),
                        fix: None,
                    });
                }
            }
        } else {
            // No logging level reported
            findings.push(Finding {
                id: "log:unknown",
                severity: Severity::Info,
                title: "Logging level unknown".into(),
                detail: "Could not determine the UFW logging level.".into(),
                fix: None,
            });
        }
    }

    findings
}

/// Check application profiles.
fn check_app_profiles(ufw: &Ufw) -> Vec<Finding> {
    let mut findings = Vec::new();

    match ufw.app_list() {
        Ok(list) => {
            if list.trim().is_empty() || list.contains("No profiles") {
                findings.push(Finding {
                    id: "app:none",
                    severity: Severity::Info,
                    title: "No application profiles".into(),
                    detail: "No UFW application profiles are installed.".into(),
                    fix: None,
                });
            } else {
                findings.push(Finding {
                    id: "app:exists",
                    severity: Severity::Ok,
                    title: "Application profiles found".into(),
                    detail: format!("Installed profiles:\n{list}"),
                    fix: None,
                });

                // Validate each profile by trying to get info
                for line in list.lines() {
                    let trimmed = line.trim();
                    // Skip header lines
                    if trimmed.is_empty() || trimmed.starts_with("Available") {
                        continue;
                    }
                    // Extract profile name (strip leading whitespace/bullets)
                    let name = trimmed.trim_start_matches(|c: char| c.is_whitespace() || c == '*' || c == '-');

                    // Check app name validity
                    if let Err(e) = validate_app_name_for_doctor(name) {
                        findings.push(Finding {
                            id: Box::leak(format!("app:name-invalid:{name}").into_boxed_str()),
                            severity: Severity::Warning,
                            title: format!("Invalid app profile name: {name}"),
                            detail: format!("App profile name '{name}' is invalid: {e}"),
                            fix: Some("Rename the profile to use only alphanumeric characters, hyphens, and underscores.".into()),
                        });
                        continue;
                    }

                    // Try to get info about the profile to validate it
                    match ufw.app_info(name) {
                        Ok(info) => {
                            // Check port specs in the info output
                            if let Some(ports_line) = info.lines().find(|l| l.contains("Port")) {
                                if let Some(ports_str) = ports_line.split(':').nth(1) {
                                    for port_spec in ports_str.split(',') {
                                        let port_spec = port_spec.trim();
                                        if !port_spec.is_empty() && !is_valid_port_spec(port_spec) {
                                            findings.push(Finding {
                                                id: Box::leak(format!("app:port-invalid:{name}:{port_spec}").into_boxed_str()),
                                                severity: Severity::Warning,
                                                title: format!("Invalid port spec in profile {name}"),
                                                detail: format!("Port spec '{port_spec}' in profile '{name}' does not match expected format (port/proto)."),
                                                fix: Some("Fix the port specification in the profile file.".into()),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            findings.push(Finding {
                                id: Box::leak(format!("app:info-fail:{name}").into_boxed_str()),
                                severity: Severity::Info,
                                title: format!("Could not read info for profile {name}"),
                                detail: format!("Failed to parse app profile '{name}'."),
                                fix: None,
                            });
                        }
                    }
                }
            }
        }
        Err(e) => findings.push(Finding {
            id: "app:list-fail",
            severity: Severity::Warning,
            title: "Could not list app profiles".into(),
            detail: format!("Failed to list app profiles: {e}"),
            fix: None,
        }),
    }

    findings
}

/// Check file permissions.
fn check_permissions(_ufw: &Ufw) -> Vec<Finding> {
    let mut findings = Vec::new();

    let paths_to_check = [
        "/etc/ufw",
        "/etc/default/ufw",
        "/etc/ufw/applications.d",
        // Backup directory (typically /etc/ufw/.bak or similar)
        "/etc/ufw/user.rules",
        "/etc/ufw/user6.rules",
        // Framework files
        "/etc/ufw/before.rules",
        "/etc/ufw/after.rules",
    ];

    for path in &paths_to_check {
        check_path_permissions(path, &mut findings);
    }

    // Check app profile directory separately for non-world-readable
    check_path_permissions("/etc/ufw/applications.d", &mut findings);

    findings
}

/// Check permissions on a single path and push findings.
fn check_path_permissions(path: &str, findings: &mut Vec<Finding>) {
    match std::fs::metadata(path) {
        Ok(meta) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = meta.permissions().mode();
                if mode & 0o002 != 0 {
                    findings.push(Finding {
                        id: Box::leak(
                            format!("perm:{}:world-writable", path.replace('/', ":"))
                                .into_boxed_str(),
                        ),
                        severity: Severity::Warning,
                        title: format!("{path} is world-writable"),
                        detail: format!(
                            "{path} has permissions {:o}, which is world-writable.",
                            mode & 0o777
                        ),
                        fix: Some(format!(
                            "Fix permissions: sudo chmod o-w {path}"
                        )),
                    });
                } else {
                    findings.push(Finding {
                        id: Box::leak(
                            format!("perm:{}:ok", path.replace('/', ":"))
                                .into_boxed_str(),
                        ),
                        severity: Severity::Ok,
                        title: format!("{path} permissions OK"),
                        detail: format!(
                            "{path} has permissions {:o}.",
                            mode & 0o777
                        ),
                        fix: None,
                    });
                }
            }
            #[cfg(not(unix))]
            {
                findings.push(Finding {
                    id: Box::leak(
                        format!("perm:{}:skip", path.replace('/', ":"))
                            .into_boxed_str(),
                    ),
                    severity: Severity::Info,
                    title: format!("Cannot check {path} permissions"),
                    detail: "Permission checks require Unix.".into(),
                    fix: None,
                });
            }
        }
        Err(_) => {
            findings.push(Finding {
                id: Box::leak(
                    format!("perm:{}:missing", path.replace('/', ":"))
                        .into_boxed_str(),
                ),
                severity: Severity::Info,
                title: format!("{path} not found"),
                detail: format!("{path} does not exist."),
                fix: None,
            });
        }
    }
}

// ── Helper functions ──────────────────────────────────────────────

/// Read whether IPv6 is enabled from /etc/default/ufw using the config module.
fn read_ipv6_enabled_from_config() -> bool {
    read_default_ufw_config().ipv6.unwrap_or(false)
}

/// Read and parse /etc/default/ufw using the config module.
fn read_default_ufw_config() -> crate::spec::UfwConfig {
    std::fs::read_to_string("/etc/default/ufw")
        .ok()
        .map(|content| crate::config::parse_default_ufw(&content))
        .unwrap_or_default()
}

/// Read and parse /etc/ufw/ufw.conf using the config module.
fn read_ufw_conf() -> crate::spec::UfwConf {
    std::fs::read_to_string("/etc/ufw/ufw.conf")
        .ok()
        .map(|content| crate::config::parse_ufw_conf(&content))
        .unwrap_or_default()
}

/// Validate an app name for doctor purposes.
/// Returns Ok if the name is valid, Err with a description otherwise.
fn validate_app_name_for_doctor(name: &str) -> std::result::Result<(), String> {
    if name.is_empty() {
        return Err("empty name".into());
    }
    if name.contains('\n') {
        return Err("contains newline".into());
    }
    if name.contains("..") || name.contains('/') {
        return Err("contains path traversal".into());
    }
    // Check for valid characters: alphanumeric, hyphens, underscores, spaces
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' || c == '.') {
        return Err("contains special characters".into());
    }
    Ok(())
}

/// Check if a port spec string is valid (e.g., "80/tcp", "443/udp", "8080:8081/tcp").
fn is_valid_port_spec(spec: &str) -> bool {
    let spec = spec.trim();
    if spec.is_empty() {
        return false;
    }

    // Must contain a / separating port from protocol
    let parts: Vec<&str> = spec.rsplitn(2, '/').collect();
    if parts.len() != 2 {
        return false;
    }

    let proto = parts[0].trim(); // after rsplit, proto comes first
    let port = parts[1].trim(); // port comes second

    // Validate protocol
    if proto != "tcp" && proto != "udp" {
        return false;
    }

    // Validate port part: single port, range, or list
    // Single port
    if let Ok(p) = port.parse::<u16>() {
        return p > 0;
    }

    // Range (e.g., "8000:9000")
    if port.contains(':') {
        let range_parts: Vec<&str> = port.split(':').collect();
        if range_parts.len() == 2 {
            if let (Ok(s), Ok(e)) = (range_parts[0].parse::<u16>(), range_parts[1].parse::<u16>()) {
                return s > 0 && e > 0 && s <= e;
            }
        }
    }

    true // Allow named ports and other formats we can't easily validate
}

#[cfg(test)]
#[path = "doctor.test.rs"]
mod tests;
