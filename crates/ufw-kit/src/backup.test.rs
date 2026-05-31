use super::*;
use crate::paths::UfwPaths;

// ---------------------------------------------------------------------------
// create_backup
// ---------------------------------------------------------------------------

#[test]
fn create_backup_should_read_existing_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Create the config files
    std::fs::create_dir_all(root.join("etc/default")).unwrap();
    std::fs::create_dir_all(root.join("etc/ufw")).unwrap();
    std::fs::write(root.join("etc/default/ufw"), "ENABLED=yes\n").unwrap();
    std::fs::write(root.join("etc/ufw/ufw.conf"), "[ufw]\nENABLED=yes\n").unwrap();

    let paths = UfwPaths::with_root(root);
    let bundle = create_backup(&paths).unwrap();

    assert!(bundle.default_ufw.is_some());
    assert!(bundle.default_ufw.unwrap().contains("ENABLED=yes"));
    assert!(bundle.ufw_conf.is_some());
}

#[test]
fn create_backup_should_handle_missing_files() {
    let dir = tempfile::tempdir().unwrap();
    let paths = UfwPaths::with_root(dir.path());
    let bundle = create_backup(&paths).unwrap();

    assert!(bundle.default_ufw.is_none());
    assert!(bundle.ufw_conf.is_none());
    assert!(bundle.sysctl_conf.is_none());
}

#[test]
fn create_backup_should_read_app_profiles() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    std::fs::create_dir_all(root.join("etc/ufw/applications.d")).unwrap();
    std::fs::write(
        root.join("etc/ufw/applications.d/myapp"),
        "[MyApp]\nports=80/tcp\n",
    )
    .unwrap();

    let paths = UfwPaths::with_root(root);
    let bundle = create_backup(&paths).unwrap();

    assert_eq!(bundle.app_profiles.len(), 1);
    assert_eq!(bundle.app_profiles[0].0, "myapp");
}

// ---------------------------------------------------------------------------
// write_backup
// ---------------------------------------------------------------------------

#[test]
fn write_backup_should_write_all_files() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = BackupBundle {
        timestamp: "12345".into(),
        default_ufw: Some("ENABLED=yes\n".into()),
        ufw_conf: None,
        sysctl_conf: None,
        app_profiles: vec![("myapp".into(), "[MyApp]\n".into())],
        framework_files: vec![("before.rules".into(), "*filter\n".into())],
    };

    let backup_dir = dir.path().join("backup");
    write_backup(&bundle, &backup_dir).unwrap();

    assert!(backup_dir.join("default-ufw").exists());
    assert!(backup_dir.join("applications.d/myapp").exists());
    assert!(backup_dir.join("framework/before.rules").exists());
}

#[test]
fn write_backup_should_handle_empty_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = BackupBundle {
        timestamp: "12345".into(),
        default_ufw: None,
        ufw_conf: None,
        sysctl_conf: None,
        app_profiles: vec![],
        framework_files: vec![],
    };

    let backup_dir = dir.path().join("backup");
    write_backup(&bundle, &backup_dir).unwrap();
}

// ---------------------------------------------------------------------------
// Edge-case tests
// ---------------------------------------------------------------------------

#[test]
fn create_backup_should_read_framework_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    std::fs::create_dir_all(root.join("etc/ufw")).unwrap();
    std::fs::write(root.join("etc/ufw/before.rules"), "*filter\n:INPUT DROP\n").unwrap();

    let paths = UfwPaths::with_root(root);
    let bundle = create_backup(&paths).unwrap();

    assert_eq!(bundle.framework_files.len(), 1);
    assert_eq!(bundle.framework_files[0].0, "before.rules");
}

// ---------------------------------------------------------------------------
// Weird edge-case tests
// ---------------------------------------------------------------------------

#[test]
fn write_backup_should_overwrite_existing() {
    let dir = tempfile::tempdir().unwrap();
    let bundle1 = BackupBundle {
        timestamp: "111".into(),
        default_ufw: Some("version1\n".into()),
        ufw_conf: None,
        sysctl_conf: None,
        app_profiles: vec![],
        framework_files: vec![],
    };
    let bundle2 = BackupBundle {
        timestamp: "222".into(),
        default_ufw: Some("version2\n".into()),
        ufw_conf: None,
        sysctl_conf: None,
        app_profiles: vec![],
        framework_files: vec![],
    };

    let backup_dir = dir.path().join("backup");
    write_backup(&bundle1, &backup_dir).unwrap();
    write_backup(&bundle2, &backup_dir).unwrap();

    let content = std::fs::read_to_string(backup_dir.join("default-ufw")).unwrap();
    assert_eq!(content, "version2\n");
}
