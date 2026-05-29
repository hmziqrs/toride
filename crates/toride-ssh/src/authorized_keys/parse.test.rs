use super::*;
use std::io::Write as IoWrite;

#[test]
fn find_key_type_no_options() {
    assert_eq!(find_key_type_offset("ssh-rsa AAAAB3Nz..."), Some(0));
}

#[test]
fn find_key_type_with_option() {
    let line = "command=\"true\" ssh-ed25519 AAAAC3Nz...";
    let offset = find_key_type_offset(line).unwrap();
    assert!(line[offset..].starts_with("ssh-ed25519 "));
}

#[test]
fn find_key_type_option_with_spaces_in_value() {
    let line = "command=\"echo hello world\" ssh-ed25519 AAAAC3Nz...";
    let offset = find_key_type_offset(line).unwrap();
    assert!(line[offset..].starts_with("ssh-ed25519 "));
}

#[test]
fn find_key_type_option_with_escaped_quotes() {
    let line = "command=\"echo \\\"hello\\\"\" ssh-ed25519 AAAAC3Nz...";
    let offset = find_key_type_offset(line).unwrap();
    assert!(line[offset..].starts_with("ssh-ed25519 "));
}

#[test]
fn find_key_type_no_key_found() {
    assert_eq!(find_key_type_offset("just some random text"), None);
}

#[test]
fn find_key_type_prefix_inside_quotes_ignored() {
    // "ssh-ed25519" inside a quoted value must NOT be detected as key type
    let line = "command=\"ssh-ed25519 is cool\" ssh-rsa AAAAB3Nz...";
    let offset = find_key_type_offset(line).unwrap();
    assert_eq!(&line[offset..offset + 7], "ssh-rsa");
}

#[tokio::test]
async fn parse_empty_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("authorized_keys");
    std::fs::write(&path, "").unwrap();
    let entries = parse_authorized_keys(&path).await.unwrap();
    assert!(entries.is_empty());
}

#[tokio::test]
async fn parse_comments_and_blanks() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("authorized_keys");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "# this is a comment").unwrap();
    writeln!(f).unwrap();
    writeln!(f, "  ").unwrap();
    let entries = parse_authorized_keys(&path).await.unwrap();
    assert!(entries.is_empty());
}
