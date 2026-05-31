//! Test support utilities for ufw-kit.
//!
//! Provides helpers for writing tests against ufw-kit without
//! requiring a real UFW installation.

pub mod fixtures;

// Re-export command types
pub use ufw_kit::command::{CommandLog, CommandRunner, FakeRunner};

// Re-export spec types for convenience
pub use ufw_kit::spec::{
    Action, Address, AppDefaultPolicy, DeleteOptions, Direction, DisableOptions, EnableOptions,
    LoggingLevel, Policy, Protocol, ProtocolFilter, ResetOptions, RouteRuleSpec, RuleLogging,
    RulePosition, RuleSpec, UfwReport,
};

/// Helper to create a fake UFW status response.
pub fn fake_status_response(active: bool, rules: &[&str]) -> String {
    if active {
        let mut out = "Status: active\n\nTo                         Action      From\n--                         ------      ----\n".to_string();
        for rule in rules {
            out.push_str(rule);
            out.push('\n');
        }
        out
    } else {
        "Status: inactive\n".to_string()
    }
}

/// Helper to create a fake UFW version response.
pub fn fake_version_response(version: &str) -> String {
    format!("ufw {version}\nCopyright (C) 2024 Canonical Ltd.\n")
}
