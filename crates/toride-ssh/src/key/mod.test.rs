use super::*;

#[test]
fn validate_key_name_empty() {
    assert!(validate_key_name("").is_err());
}

#[test]
fn validate_key_name_slash() {
    assert!(validate_key_name("../etc/passwd").is_err());
}

#[test]
fn validate_key_name_backslash() {
    assert!(validate_key_name("..\\etc\\passwd").is_err());
}

#[test]
fn validate_key_name_dot_dot() {
    assert!(validate_key_name("../../etc/passwd").is_err());
}

#[test]
fn validate_key_name_dot_dot_in_middle() {
    assert!(validate_key_name("foo/../bar").is_err());
}

#[test]
fn validate_key_name_valid() {
    assert!(validate_key_name("id_ed25519").is_ok());
    assert!(validate_key_name("my-key").is_ok());
    assert!(validate_key_name("key_with_underscores").is_ok());
}

#[test]
fn validate_key_name_dot_file() {
    // Dot files like ".ssh" should be valid (no path traversal)
    assert!(validate_key_name(".hidden").is_ok());
}

#[test]
fn validate_key_name_unicode() {
    // Unicode names should be valid as long as no path traversal
    assert!(validate_key_name("clé").is_ok());
}

#[test]
fn validate_key_name_just_dot_dot() {
    assert!(validate_key_name("..").is_err());
}
