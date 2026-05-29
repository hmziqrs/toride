use super::*;

#[test]
fn parse_algorithm_ed25519() {
    assert_eq!(
        parse_key_type_from_algorithm("ssh-ed25519"),
        Some(KeyType::Ed25519)
    );
}

#[test]
fn parse_algorithm_ecdsa() {
    assert_eq!(
        parse_key_type_from_algorithm("ecdsa-sha2-nistp384"),
        Some(KeyType::EcdsaP384)
    );
}

#[test]
fn parse_algorithm_unknown_returns_none() {
    assert_eq!(parse_key_type_from_algorithm("unknown-algo"), None);
}

#[test]
fn parse_algorithm_empty_string_returns_none() {
    assert_eq!(parse_key_type_from_algorithm(""), None);
}

#[test]
fn parse_algorithm_all_known_types() {
    assert_eq!(parse_key_type_from_algorithm("ssh-ed25519"), Some(KeyType::Ed25519));
    assert_eq!(parse_key_type_from_algorithm("ssh-rsa"), Some(KeyType::Rsa { bits: 0 }));
    assert_eq!(parse_key_type_from_algorithm("ecdsa-sha2-nistp256"), Some(KeyType::EcdsaP256));
    assert_eq!(parse_key_type_from_algorithm("ecdsa-sha2-nistp384"), Some(KeyType::EcdsaP384));
    assert_eq!(parse_key_type_from_algorithm("ecdsa-sha2-nistp521"), Some(KeyType::EcdsaP521));
    assert_eq!(parse_key_type_from_algorithm("ssh-dss"), Some(KeyType::Dsa));
    assert_eq!(parse_key_type_from_algorithm("sk-ssh-ed25519@openssh.com"), Some(KeyType::SkEd25519));
    assert_eq!(parse_key_type_from_algorithm("sk-ecdsa-sha2-nistp256@openssh.com"), Some(KeyType::SkEcdsaP256));
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

#[test]
fn parse_ssh_add_empty_line() {
    assert!(parse_ssh_add_line("").is_none());
}

#[test]
fn parse_ssh_add_whitespace_only() {
    assert!(parse_ssh_add_line("   ").is_none());
}

#[test]
fn parse_ssh_add_unknown_type_returns_none() {
    // Unknown key type in parentheses should return None
    let line = "256 SHA256:AAAA comment (UNKNOWN-TYPE)";
    assert!(parse_ssh_add_line(line).is_none());
}

#[test]
fn parse_ssh_add_ecdsa_sk() {
    let line = "256 SHA256:AAAA user@host (ECDSA-SK)";
    let key = parse_ssh_add_line(line).unwrap();
    assert_eq!(key.key_type, KeyType::SkEcdsaP256);
    assert_eq!(key.comment.as_deref(), Some("user@host"));
}

#[test]
fn parse_ssh_add_ed25519_sk() {
    let line = "256 SHA256:AAAA user@host (ED25519-SK)";
    let key = parse_ssh_add_line(line).unwrap();
    assert_eq!(key.key_type, KeyType::SkEd25519);
}

#[test]
fn parse_ssh_add_dsa() {
    let line = "1024 SHA256:AAAA user@host (DSA)";
    let key = parse_ssh_add_line(line).unwrap();
    assert_eq!(key.key_type, KeyType::Dsa);
}

#[test]
fn parse_ssh_add_agent_source() {
    let line = "256 SHA256:AAAA comment (ED25519)";
    let key = parse_ssh_add_line(line).unwrap();
    assert_eq!(key.source, KeySource::Agent);
    assert!(!key.encrypted);
    assert!(!key.has_public_pair);
    assert!(!key.has_certificate);
}

#[test]
fn parse_ssh_add_fingerprint_format() {
    let line = "256 SHA256:abcdef1234567890 comment (ED25519)";
    let key = parse_ssh_add_line(line).unwrap();
    let fp = key.fingerprint.unwrap();
    assert_eq!(fp.hash, "abcdef1234567890");
    assert_eq!(fp.key_type, KeyType::Ed25519);
}

#[test]
fn parse_ssh_add_path_is_agent_identifier() {
    let line = "256 SHA256:AAAA my-key (ED25519)";
    let key = parse_ssh_add_line(line).unwrap();
    assert!(key.path.to_str().unwrap().starts_with("agent:"));
}
