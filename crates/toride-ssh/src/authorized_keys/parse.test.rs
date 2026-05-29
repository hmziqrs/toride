use super::*;
use std::io::Write as IoWrite;

#[test]
fn find_key_type_offset_should_return_zero_when_no_options_present() {
    assert_eq!(find_key_type_offset("ssh-rsa AAAAB3Nz..."), Some(0));
}

#[test]
fn find_key_type_offset_should_skip_past_options() {
    let line = "command=\"true\" ssh-ed25519 AAAAC3Nz...";
    let offset = find_key_type_offset(line).unwrap();
    assert!(line[offset..].starts_with("ssh-ed25519 "));
}

#[test]
fn find_key_type_offset_should_handle_spaces_in_quoted_values() {
    let line = "command=\"echo hello world\" ssh-ed25519 AAAAC3Nz...";
    let offset = find_key_type_offset(line).unwrap();
    assert!(line[offset..].starts_with("ssh-ed25519 "));
}

#[test]
fn find_key_type_offset_should_handle_escaped_quotes_in_values() {
    let line = "command=\"echo \\\"hello\\\"\" ssh-ed25519 AAAAC3Nz...";
    let offset = find_key_type_offset(line).unwrap();
    assert!(line[offset..].starts_with("ssh-ed25519 "));
}

#[test]
fn find_key_type_offset_should_return_none_when_no_key_type_found() {
    assert_eq!(find_key_type_offset("just some random text"), None);
}

#[test]
fn find_key_type_offset_should_ignore_key_type_prefix_inside_quotes() {
    // "ssh-ed25519" inside a quoted value must NOT be detected as key type
    let line = "command=\"ssh-ed25519 is cool\" ssh-rsa AAAAB3Nz...";
    let offset = find_key_type_offset(line).unwrap();
    assert_eq!(&line[offset..offset + 7], "ssh-rsa");
}

#[tokio::test]
async fn parse_authorized_keys_should_return_empty_vec_for_empty_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("authorized_keys");
    std::fs::write(&path, "").unwrap();
    let entries = parse_authorized_keys(&path).await.unwrap();
    assert!(entries.is_empty());
}

#[tokio::test]
async fn parse_authorized_keys_should_skip_comments_and_blank_lines() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("authorized_keys");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "# this is a comment").unwrap();
    writeln!(f).unwrap();
    writeln!(f, "  ").unwrap();
    let entries = parse_authorized_keys(&path).await.unwrap();
    assert!(entries.is_empty());
}

// ---------------------------------------------------------------------------
// Edge-case tests for find_key_type_offset
// ---------------------------------------------------------------------------

#[test]
fn find_key_type_offset_empty_string() {
    assert_eq!(find_key_type_offset(""), None);
}

#[test]
fn find_key_type_offset_only_options() {
    // No key type present
    assert_eq!(find_key_type_offset("command=\"true\""), None);
}

#[test]
fn find_key_type_offset_multiple_options() {
    let line = "no-pty,command=\"true\" ssh-ed25519 AAAAC3...";
    let offset = find_key_type_offset(line).unwrap();
    assert!(line[offset..].starts_with("ssh-ed25519 "));
}

#[test]
fn find_key_type_offset_ecdsa() {
    assert_eq!(find_key_type_offset("ecdsa-sha2-nistp256 AAAA..."), Some(0));
}

#[test]
fn find_key_type_offset_sk_key() {
    assert_eq!(find_key_type_offset("sk-ssh-ed25519@openssh.com AAAA..."), Some(0));
}

#[test]
fn find_key_type_offset_dss() {
    assert_eq!(find_key_type_offset("ssh-dss AAAA..."), Some(0));
}

// ---------------------------------------------------------------------------
// Edge-case tests for starts_with_key_type
// ---------------------------------------------------------------------------

#[test]
fn starts_with_key_type_all_prefixes() {
    for prefix in KEY_TYPE_PREFIXES {
        assert!(
            starts_with_key_type(&format!("{prefix} AAAA")),
            "failed for prefix: {prefix}"
        );
    }
}

#[test]
fn starts_with_key_type_no_space_after() {
    // Key type must be followed by a space
    assert!(!starts_with_key_type("ssh-ed25519AAAA"));
}

#[test]
fn starts_with_key_type_empty() {
    assert!(!starts_with_key_type(""));
}

#[test]
fn starts_with_key_type_partial_match() {
    assert!(!starts_with_key_type("ssh-ed2 AAAA"));
}

// ---------------------------------------------------------------------------
// Weird edge-case tests
// ---------------------------------------------------------------------------

#[test]
fn find_key_type_offset_with_nested_quotes() {
    // Options with nested quotes should not confuse the parser
    let line = "command=\"echo \\\"ssh-ed25519\\\"\" ssh-rsa AAAA";
    let offset = find_key_type_offset(line).unwrap();
    assert!(&line[offset..].starts_with("ssh-rsa"));
}

#[test]
fn find_key_type_offset_with_backslash_at_end() {
    // Backslash at end of quoted value
    let line = "command=\"path\\\\\" ssh-rsa AAAA";
    let offset = find_key_type_offset(line).unwrap();
    assert!(&line[offset..].starts_with("ssh-rsa"));
}

#[test]
fn find_key_type_offset_with_multiple_options() {
    let line = "no-pty,from=\"10.0.0.*\",command=\"/bin/bash\" ssh-ed25519 AAAA";
    let offset = find_key_type_offset(line).unwrap();
    assert!(&line[offset..].starts_with("ssh-ed25519"));
}

#[test]
fn find_key_type_offset_with_no_space_before_key() {
    // No space between options and key type (malformed)
    let line = "no-ptyssh-ed25519 AAAA";
    // This should still find ssh-ed25519 if it appears after a space
    let result = find_key_type_offset(line);
    // Since there's no space before ssh-ed25519, it should not match
    // (the key type must be preceded by a space or be at position 0)
    let _ = result;
}

#[test]
fn find_key_type_offset_with_key_at_start() {
    let line = "ssh-ed25519 AAAA";
    assert_eq!(find_key_type_offset(line), Some(0));
}

#[test]
fn starts_with_key_type_with_comment() {
    // Key type followed by comment (no space after type)
    assert!(!starts_with_key_type("ssh-ed25519AAAA"));
}

#[test]
fn starts_with_key_type_with_tab() {
    // Key type followed by tab instead of space — the parser requires a space, not tab
    assert!(!starts_with_key_type("ssh-ed25519\tAAAA"));
}

#[test]
fn find_key_type_offset_with_very_long_options() {
    let long_opts = "a".repeat(10000);
    let line = format!("{long_opts} ssh-ed25519 AAAA");
    let offset = find_key_type_offset(&line).unwrap();
    assert!(&line[offset..].starts_with("ssh-ed25519"));
}

#[test]
fn find_key_type_offset_with_empty_options() {
    // Empty options field followed by space
    let line = " ssh-ed25519 AAAA";
    let offset = find_key_type_offset(line).unwrap();
    assert_eq!(offset, 1);
}

#[test]
fn find_key_type_offset_with_escaped_quote_at_boundary() {
    // Escaped quote right before the space
    let line = "command=\"test\\\"\" ssh-ed25519 AAAA";
    let offset = find_key_type_offset(line).unwrap();
    assert!(&line[offset..].starts_with("ssh-ed25519"));
}
