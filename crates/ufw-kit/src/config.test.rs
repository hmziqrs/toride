use super::*;
use std::fs;

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

// ---------------------------------------------------------------------------
// parse_ufw_conf
// ---------------------------------------------------------------------------

#[test]
fn parse_ufw_conf_should_parse_enabled() {
    let content = "# /etc/ufw/ufw.conf\nENABLED=yes\n";
    let conf = parse_ufw_conf(content);
    assert_eq!(conf.enabled, Some(true));
}

#[test]
fn parse_ufw_conf_should_parse_disabled() {
    let content = "ENABLED=no\n";
    let conf = parse_ufw_conf(content);
    assert_eq!(conf.enabled, Some(false));
}

#[test]
fn parse_ufw_conf_should_parse_loglevel() {
    let content = "ENABLED=yes\nLOGLEVEL=low\n";
    let conf = parse_ufw_conf(content);
    assert_eq!(conf.enabled, Some(true));
    assert_eq!(conf.loglevel, Some("low".to_string()));
}

#[test]
fn parse_ufw_conf_should_handle_empty() {
    let conf = parse_ufw_conf("");
    assert!(conf.enabled.is_none());
    assert!(conf.loglevel.is_none());
}

#[test]
fn parse_ufw_conf_should_skip_comments() {
    let content = "# /etc/ufw/ufw.conf\n#\nENABLED=yes\nLOGLEVEL=low\n";
    let conf = parse_ufw_conf(content);
    assert_eq!(conf.enabled, Some(true));
    assert_eq!(conf.loglevel, Some("low".to_string()));
}

#[test]
fn parse_ufw_conf_should_ignore_unknown_keys() {
    let content = "ENABLED=yes\nUNKNOWN_KEY=value\n";
    let conf = parse_ufw_conf(content);
    assert_eq!(conf.enabled, Some(true));
    assert!(conf.loglevel.is_none());
}

// ---------------------------------------------------------------------------
// update_ufw_conf_key
// ---------------------------------------------------------------------------

#[test]
fn update_ufw_conf_key_should_update_existing() {
    let content = "ENABLED=no\nLOGLEVEL=low\n";
    let updated = update_ufw_conf_key(content, "ENABLED", "yes");
    assert!(updated.contains("ENABLED=yes"));
    assert!(updated.contains("LOGLEVEL=low"));
}

#[test]
fn update_ufw_conf_key_should_add_missing() {
    let content = "ENABLED=yes\n";
    let updated = update_ufw_conf_key(content, "LOGLEVEL", "medium");
    assert!(updated.contains("ENABLED=yes"));
    assert!(updated.contains("LOGLEVEL=medium"));
}

#[test]
fn update_ufw_conf_key_should_preserve_comments() {
    let content = "# /etc/ufw/ufw.conf\nENABLED=no\n";
    let updated = update_ufw_conf_key(content, "ENABLED", "yes");
    assert!(updated.contains("# /etc/ufw/ufw.conf"));
    assert!(updated.contains("ENABLED=yes"));
}

// ---------------------------------------------------------------------------
// write_config_file
// ---------------------------------------------------------------------------

#[test]
fn write_config_file_should_create_new_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.conf");
    write_config_file(&path, "ENABLED=yes\n", None).unwrap();
    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content, "ENABLED=yes\n");
}

#[test]
fn write_config_file_should_overwrite_existing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.conf");
    fs::write(&path, "ENABLED=no\n").unwrap();
    write_config_file(&path, "ENABLED=yes\n", None).unwrap();
    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content, "ENABLED=yes\n");
}

#[test]
fn write_config_file_should_create_backup() {
    let dir = tempfile::tempdir().unwrap();
    let backup_dir = dir.path().join("backups");
    let path = dir.path().join("test.conf");

    // Write original content
    fs::write(&path, "ENABLED=no\n").unwrap();

    // Write new content with backup
    write_config_file(&path, "ENABLED=yes\n", Some(&backup_dir)).unwrap();

    // Original should be updated
    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content, "ENABLED=yes\n");

    // Backup should exist with original content
    let backup_path = backup_dir.join("test.conf.bak");
    assert!(backup_path.exists());
    let backup_content = fs::read_to_string(&backup_path).unwrap();
    assert_eq!(backup_content, "ENABLED=no\n");
}

#[test]
fn write_config_file_should_not_backup_if_file_missing() {
    let dir = tempfile::tempdir().unwrap();
    let backup_dir = dir.path().join("backups");
    let path = dir.path().join("new.conf");

    write_config_file(&path, "ENABLED=yes\n", Some(&backup_dir)).unwrap();

    // File should exist
    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content, "ENABLED=yes\n");

    // No backup should exist (original didn't exist)
    let backup_path = backup_dir.join("new.conf.bak");
    assert!(!backup_path.exists());
}

#[test]
fn write_config_file_should_create_parent_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("subdir/nested/test.conf");

    write_config_file(&path, "ENABLED=yes\n", None).unwrap();
    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content, "ENABLED=yes\n");
}
