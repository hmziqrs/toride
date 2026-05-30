use super::*;

// ---------------------------------------------------------------------------
// parse_default_ufw
// ---------------------------------------------------------------------------

#[test]
fn parse_default_ufw_should_parse_ipv6() {
    let content = "IPV6=yes\n";
    let config = parse_default_ufw(content);
    assert_eq!(config.ipv6, Some(true));
}

#[test]
fn parse_default_ufw_should_parse_ipv6_no() {
    let content = "IPV6=no\n";
    let config = parse_default_ufw(content);
    assert_eq!(config.ipv6, Some(false));
}

#[test]
fn parse_default_ufw_should_parse_policies() {
    let content = "\
DEFAULT_INPUT_POLICY=DROP
DEFAULT_OUTPUT_POLICY=ACCEPT
DEFAULT_FORWARD_POLICY=DROP
";
    let config = parse_default_ufw(content);
    assert_eq!(config.default_input_policy, Some("DROP".into()));
    assert_eq!(config.default_output_policy, Some("ACCEPT".into()));
    assert_eq!(config.default_forward_policy, Some("DROP".into()));
}

#[test]
fn parse_default_ufw_should_parse_enabled() {
    let content = "ENABLED=yes\n";
    let config = parse_default_ufw(content);
    assert_eq!(config.enabled, Some(true));
}

#[test]
fn parse_default_ufw_should_skip_comments() {
    let content = "# This is a comment\nIPV6=yes\n";
    let config = parse_default_ufw(content);
    assert_eq!(config.ipv6, Some(true));
}

#[test]
fn parse_default_ufw_should_handle_quoted_values() {
    let content = "IPT_SYSCTL=/etc/ufw/sysctl.conf\n";
    let config = parse_default_ufw(content);
    assert_eq!(config.ipt_sysctl, Some("/etc/ufw/sysctl.conf".into()));
}

// ---------------------------------------------------------------------------
// update_config_key
// ---------------------------------------------------------------------------

#[test]
fn update_config_key_should_update_existing() {
    let content = "IPV6=no\nENABLED=yes\n";
    let updated = update_config_key(content, "IPV6", "yes");
    assert!(updated.contains("IPV6=yes"));
    assert!(updated.contains("ENABLED=yes"));
    // Original value should be replaced
    assert!(!updated.contains("IPV6=no\n"));
}

#[test]
fn update_config_key_should_add_missing() {
    let content = "ENABLED=yes\n";
    let updated = update_config_key(content, "IPV6", "yes");
    assert!(updated.contains("IPV6=yes"));
    assert!(updated.contains("ENABLED=yes"));
}

#[test]
fn update_config_key_should_preserve_comments() {
    let content = "# UFW config\nIPV6=no\nENABLED=yes\n";
    let updated = update_config_key(content, "IPV6", "yes");
    assert!(updated.contains("# UFW config"));
    assert!(updated.contains("IPV6=yes"));
}

// ---------------------------------------------------------------------------
// Edge-case tests
// ---------------------------------------------------------------------------

#[test]
fn parse_default_ufw_should_handle_empty_content() {
    let config = parse_default_ufw("");
    assert!(config.ipv6.is_none());
    assert!(config.enabled.is_none());
}

#[test]
fn parse_default_ufw_should_handle_manage_builtins() {
    let content = "MANAGE_BUILTINS=yes\n";
    let config = parse_default_ufw(content);
    assert_eq!(config.manage_builtins, Some(true));
}

// ---------------------------------------------------------------------------
// Weird edge-case tests
// ---------------------------------------------------------------------------

#[test]
fn parse_default_ufw_should_handle_values_with_quotes() {
    let content = "IPV6=\"yes\"\n";
    let config = parse_default_ufw(content);
    assert_eq!(config.ipv6, Some(true));
}

#[test]
fn update_config_key_should_handle_empty_content() {
    let updated = update_config_key("", "IPV6", "yes");
    assert!(updated.contains("IPV6=yes"));
}

#[test]
fn parse_default_ufw_should_skip_semicolon_comments() {
    let content = "; comment\nIPV6=yes\n";
    let config = parse_default_ufw(content);
    assert_eq!(config.ipv6, Some(true));
}
