//! Rendering functions for firewall rules and security groups.
//!
//! Converts typed domain objects into human-readable or provider-specific
//! formats for display, logging, or configuration file generation.

use crate::spec::{FirewallRule, SecurityGroup};

// ---------------------------------------------------------------------------
// Human-readable rendering
// ---------------------------------------------------------------------------

/// Render a single firewall rule as a human-readable string.
///
/// Format: `INGRESS  tcp  0.0.0.0/0  80  allow  "HTTP traffic"`
pub fn render_firewall_rule(rule: &FirewallRule) -> String {
    let direction = if rule.is_ingress { "INGRESS" } else { "EGRESS" };
    let port = match &rule.port_range {
        Some(pr) => pr.to_string(),
        None => "all".to_string(),
    };
    let action = rule.action;
    let desc = if rule.description.is_empty() {
        String::new()
    } else {
        format!("  \"{}\"", rule.description)
    };
    format!(
        "{direction}  {proto}  {cidr}  {port}  {action}{desc}",
        proto = rule.protocol,
        cidr = rule.cidr,
        action = action,
    )
}

/// Render a security group as a human-readable table.
///
/// Includes a header with the group name, description, and rule count,
/// followed by each rule rendered via [`render_firewall_rule`].
pub fn render_security_group(group: &SecurityGroup) -> String {
    let mut out = String::new();
    out.push_str(&format!("Security Group: {}\n", group.name));
    if !group.description.is_empty() {
        out.push_str(&format!("  Description: {}\n", group.description));
    }
    out.push_str(&format!("  Provider: {}\n", group.provider));
    out.push_str(&format!("  Rules: {}\n", group.rules.len()));

    if !group.rules.is_empty() {
        out.push_str("\n");
        for rule in &group.rules {
            out.push_str(&format!("  {}\n", render_firewall_rule(rule)));
        }
    }

    out
}

/// Render a collection of firewall rules as a table.
///
/// Returns a string with one rule per line, sorted by direction (ingress first),
/// then protocol, then port.
pub fn render_firewall_rules(rules: &[FirewallRule]) -> String {
    let mut sorted: Vec<&FirewallRule> = rules.iter().collect();
    sorted.sort_by(|a, b| {
        a.is_ingress
            .cmp(&b.is_ingress)
            .reverse()
            .then_with(|| a.protocol.to_string().cmp(&b.protocol.to_string()))
            .then_with(|| {
                a.port_range
                    .map(|p| p.start)
                    .cmp(&b.port_range.map(|p| p.start))
            })
    });

    sorted
        .iter()
        .map(|r| render_firewall_rule(r))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// Provider-specific rendering
// ---------------------------------------------------------------------------

/// Render a firewall rule as an AWS CLI `--ip-permissions` argument.
///
/// This is a skeleton; full implementation would produce JSON for
/// `aws ec2 authorize-security-group-ingress`.
pub fn render_aws_rule(rule: &FirewallRule) -> String {
    let _ = rule;
    // TODO: Implement AWS CLI argument rendering.
    String::new()
}

/// Render a firewall rule as a GCP `gcloud compute firewall-rules create` argument.
///
/// This is a skeleton; full implementation would produce the full command line.
pub fn render_gcp_rule(rule: &FirewallRule) -> String {
    let _ = rule;
    // TODO: Implement GCP CLI argument rendering.
    String::new()
}
