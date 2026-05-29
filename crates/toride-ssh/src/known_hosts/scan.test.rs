use super::*;

#[test]
fn parse_valid_keyscan_line() {
    let line = "example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl";
    let key = parse_keyscan_line("example.com", line).unwrap();
    assert_eq!(key.host, "example.com");
    assert_eq!(key.raw_host, "example.com");
    assert_eq!(key.key_type, "ssh-ed25519");
    assert_eq!(key.public_key, "AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl");
}

#[test]
fn parse_hashed_keyscan_line_preserves_original_host() {
    let line = "|1|JfKTdBh7rNbXkVAQCRp4OQoPfmI=|USECr3SWf1JUPsms5AqfD5QfxkM= ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl";
    let key = parse_keyscan_line("example.com", line).unwrap();
    assert_eq!(key.host, "example.com");
    assert_eq!(key.raw_host, "|1|JfKTdBh7rNbXkVAQCRp4OQoPfmI=|USECr3SWf1JUPsms5AqfD5QfxkM=");
}

#[test]
fn reject_malformed_line() {
    assert!(parse_keyscan_line("host", "only-one-field").is_err());
    assert!(parse_keyscan_line("host", "two fields").is_err());
}
