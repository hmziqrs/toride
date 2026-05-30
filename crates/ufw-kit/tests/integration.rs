//! Integration tests for ufw-kit using fixture files.

use std::fs;

fn fixture(path: &str) -> String {
    let base = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/");
    fs::read_to_string(format!("{base}{path}"))
        .unwrap_or_else(|e| panic!("fixture not found: {path}: {e}"))
}

#[test]
fn parse_active_verbose_fixture() {
    let raw = fixture("status/active_verbose.txt");
    let status = ufw_kit::status::parse_status_verbose(&raw).expect("should parse");
    assert!(status.active);
    assert_eq!(status.rules.len(), 6);
}

#[test]
fn parse_inactive_fixture() {
    let raw = fixture("status/inactive.txt");
    let status = ufw_kit::status::parse_status(&raw).expect("should parse");
    assert!(!status.active);
}

#[test]
fn parse_numbered_fixture() {
    let raw = fixture("status/numbered.txt");
    let result = ufw_kit::status::parse_status_numbered(&raw).expect("should parse");
    assert!(result.active);
    assert_eq!(result.rules.len(), 4);
    assert_eq!(result.rules[0].number, Some(1));
}

#[test]
fn parse_show_listening_fixture() {
    let raw = fixture("status/show_listening.txt");
    let ports = ufw_kit::status::parse_show_listening(&raw);
    assert_eq!(ports.len(), 4);
    assert_eq!(ports[0].proto, "tcp");
    assert_eq!(ports[0].address, "0.0.0.0:22");
}

#[test]
fn parse_show_added_fixture() {
    let raw = fixture("status/show_added.txt");
    let rules = ufw_kit::status::parse_show_added(&raw);
    assert_eq!(rules.len(), 4);
    assert!(rules[0].raw.contains("22/tcp"));
}

#[test]
fn parse_app_profile_fixture() {
    let raw = fixture("app_profiles/web.txt");
    let profile = ufw_kit::app_profile::parse_profile("WebServer", &raw).expect("should parse");
    assert_eq!(profile.name, "WebServer");
    assert_eq!(profile.ports.len(), 2);
}

#[test]
fn dry_run_fixture_content() {
    let raw = fixture("dry_run/allow_22.txt");
    assert!(raw.contains("allow 22/tcp"));
}

#[test]
fn framework_fixture_has_managed_block() {
    let raw = fixture("framework/before_rules_sample.txt");
    assert!(raw.contains("Managed by ufw-kit: custom-ping"));
}
