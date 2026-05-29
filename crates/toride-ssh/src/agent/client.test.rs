use super::*;

#[test]
fn parse_algorithm_ed25519() {
    assert_eq!(
        parse_key_type_from_algorithm("ssh-ed25519"),
        KeyType::Ed25519
    );
}

#[test]
fn parse_algorithm_ecdsa() {
    assert_eq!(
        parse_key_type_from_algorithm("ecdsa-sha2-nistp384"),
        KeyType::EcdsaP384
    );
}

#[test]
fn parse_ssh_add_line_ed25519() {
    let line = "256 SHA256:ABCDEFGH1234567890 comment here (ED25519)";
    let key = parse_ssh_add_line(line).unwrap();
    assert_eq!(key.key_type, KeyType::Ed25519);
    assert_eq!(key.comment.as_deref(), Some("comment here"));
}

#[test]
fn parse_ssh_add_line_rsa_no_comment() {
    let line = "2048 SHA256:xyz123  (RSA)";
    let key = parse_ssh_add_line(line).unwrap();
    assert!(matches!(key.key_type, KeyType::Rsa { .. }));
    assert!(key.comment.is_none());
}

#[test]
fn parse_ssh_add_no_identities() {
    assert!(parse_ssh_add_line("The agent has no identities").is_none());
}
