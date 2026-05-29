use super::*;

#[test]
fn parse_empty() {
    let opts = parse_options("").unwrap();
    assert!(opts.command.is_none());
    assert!(opts.from.is_empty());
    assert!(!opts.no_pty);
}

#[test]
fn parse_boolean_flags() {
    let opts = parse_options(
        "no-pty,no-port-forwarding,no-X11-forwarding,no-agent-forwarding,no-user-rc",
    )
    .unwrap();
    assert!(opts.no_pty);
    assert!(opts.no_port_forwarding);
    assert!(opts.no_x11_forwarding);
    assert!(opts.no_agent_forwarding);
    assert!(opts.no_user_rc);
}

#[test]
fn parse_command_option() {
    let opts = parse_options("command=\"/usr/bin/true\"").unwrap();
    assert_eq!(opts.command.as_deref(), Some("/usr/bin/true"));
}

#[test]
fn parse_multiple_options() {
    let opts = parse_options("no-pty,command=\"/bin/bash\",from=\"10.0.0.*\"").unwrap();
    assert!(opts.no_pty);
    assert_eq!(opts.command.as_deref(), Some("/bin/bash"));
    assert_eq!(opts.from, vec!["10.0.0.*"]);
}

#[test]
fn parse_cert_authority() {
    let opts = parse_options("cert-authority").unwrap();
    assert!(opts.cert_authority);
}

#[test]
fn parse_environment_option() {
    let opts = parse_options("environment=\"FOO=bar\"").unwrap();
    assert_eq!(
        opts.environment,
        vec![("FOO".to_string(), "bar".to_string())]
    );
}

#[test]
fn parse_permit_open() {
    let opts = parse_options("permit-open=\"host:22\"").unwrap();
    assert_eq!(opts.permit_open, vec!["host:22"]);
}

#[test]
fn parse_custom_options() {
    let opts = parse_options("custom-flag,custom-value=\"hello\"").unwrap();
    assert_eq!(opts.custom.len(), 2);
    assert_eq!(opts.custom[0], ("custom-flag".to_string(), None));
    assert_eq!(
        opts.custom[1],
        ("custom-value".to_string(), Some("hello".to_string()))
    );
}

#[test]
fn parse_restrict_option() {
    let opts = parse_options("restrict").unwrap();
    assert!(opts.restrict);
}

#[test]
fn parse_principals_option() {
    let opts = parse_options("principals=\"admin,deploy\"").unwrap();
    assert_eq!(opts.principals, vec!["admin,deploy"]);
}

#[test]
fn parse_expiry_time_option() {
    let opts = parse_options("expiry-time=\"20250101T000000\"").unwrap();
    assert_eq!(opts.expiry_time.as_deref(), Some("20250101T000000"));
}

#[test]
fn parse_perferrp_option() {
    let opts = parse_options("perferrp").unwrap();
    assert!(opts.perferrp);
}

#[test]
fn parse_empty_command_value() {
    let opts = parse_options("command=\"\"").unwrap();
    assert_eq!(opts.command.as_deref(), Some(""));
}

#[test]
fn parse_escaped_quotes_in_command() {
    let opts = parse_options("command=\"echo \\\"hello\\\"\"").unwrap();
    assert_eq!(opts.command.as_deref(), Some("echo \"hello\""));
}

#[test]
fn parse_escaped_backslash_in_value() {
    let opts = parse_options("command=\"path\\\\to\\\\file\"").unwrap();
    assert_eq!(opts.command.as_deref(), Some("path\\to\\file"));
}

#[test]
fn parse_command_with_single_quotes() {
    // Single quotes inside double-quoted values are literal
    let opts = parse_options("command=\"echo 'hello'\"").unwrap();
    assert_eq!(opts.command.as_deref(), Some("echo 'hello'"));
}

#[test]
fn parse_command_with_comma_in_quotes() {
    let opts = parse_options("command=\"a,b\",no-pty").unwrap();
    assert_eq!(opts.command.as_deref(), Some("a,b"));
    assert!(opts.no_pty);
}

#[test]
fn unescape_only_backslash_quote() {
    assert_eq!(unescape("hello\\\"world"), "hello\"world");
}

#[test]
fn unescape_only_backslash_backslash() {
    assert_eq!(unescape("hello\\\\world"), "hello\\world");
}

#[test]
fn unescape_trailing_backslash() {
    assert_eq!(unescape("hello\\"), "hello\\");
}
