use super::*;
use crate::paths::SshPaths;
use crate::types::Severity;

/// Set up a temporary SSH directory with the given layout and run `run_all`.
async fn run_checks_with_dir(ssh_dir: &std::path::Path) -> Vec<crate::types::Diagnostic> {
    let paths = SshPaths::with_dir(ssh_dir);
    run_all(&paths).await.unwrap()
}

fn find<'a>(
    diagnostics: &'a [crate::types::Diagnostic],
    id: &str,
) -> Vec<&'a crate::types::Diagnostic> {
    diagnostics.iter().filter(|d| d.id == id).collect()
}

// ---------------------------------------------------------------------------
// SshDirExists
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ssh_dir_exists_when_present() {
    let dir = tempfile::tempdir().unwrap();
    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "ssh_dir_exists");
    assert!(!matches.is_empty());
    assert!(matches.iter().all(|d| d.severity == Severity::Ok));
}

#[tokio::test]
async fn ssh_dir_missing_when_absent() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nonexistent_ssh");
    let diags = run_checks_with_dir(&missing).await;
    let matches = find(&diags, "ssh_dir_exists");
    assert!(!matches.is_empty());
    // When the directory doesn't exist, severity should be Warning.
    assert!(matches.iter().all(|d| d.severity == Severity::Warning));
}

// ---------------------------------------------------------------------------
// SshDirPermissions (unix only)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[tokio::test]
async fn ssh_dir_permissions_correct() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "ssh_dir_permissions");
    assert!(!matches.is_empty());
    assert!(matches.iter().all(|d| d.severity == Severity::Ok));
}

#[cfg(unix)]
#[tokio::test]
async fn ssh_dir_permissions_too_permissive() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "ssh_dir_permissions");
    assert!(!matches.is_empty());
    assert!(matches.iter().all(|d| d.severity == Severity::Error));
}

// ---------------------------------------------------------------------------
// ConfigExists
// ---------------------------------------------------------------------------

#[tokio::test]
async fn config_exists_when_present() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("config"), "").unwrap();
    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "config_exists");
    assert!(!matches.is_empty());
    assert!(matches.iter().all(|d| d.severity == Severity::Ok));
}

#[tokio::test]
async fn config_missing_reports_info() {
    let dir = tempfile::tempdir().unwrap();
    // No config file written.
    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "config_exists");
    assert!(!matches.is_empty());
    assert!(matches.iter().all(|d| d.severity == Severity::Info));
}

// ---------------------------------------------------------------------------
// KnownHostsExists
// ---------------------------------------------------------------------------

#[tokio::test]
async fn known_hosts_exists_when_present() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("known_hosts"), "").unwrap();
    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "known_hosts_exists");
    assert!(!matches.is_empty());
    assert!(matches.iter().all(|d| d.severity == Severity::Ok));
}

#[tokio::test]
async fn known_hosts_missing_warns() {
    let dir = tempfile::tempdir().unwrap();
    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "known_hosts_exists");
    assert!(!matches.is_empty());
    assert!(matches.iter().all(|d| d.severity == Severity::Warning));
}

// ---------------------------------------------------------------------------
// PrivateKeyPermissions (unix only)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[tokio::test]
async fn private_key_correct_permissions() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("id_ed25519");
    std::fs::write(
        &key_path,
        "-----BEGIN OPENSSH PRIVATE KEY-----\ntest\n-----END OPENSSH PRIVATE KEY-----\n",
    )
    .unwrap();
    std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600)).unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "private_key_permissions");
    // Should have at least one Ok entry for our key.
    assert!(matches.iter().any(|d| d.severity == Severity::Ok && d.message.contains("id_ed25519")));
}

#[cfg(unix)]
#[tokio::test]
async fn private_key_wrong_permissions() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("id_ed25519");
    std::fs::write(
        &key_path,
        "-----BEGIN OPENSSH PRIVATE KEY-----\ntest\n-----END OPENSSH PRIVATE KEY-----\n",
    )
    .unwrap();
    std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o644)).unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "private_key_permissions");
    assert!(matches.iter().any(|d| d.severity == Severity::Error && d.message.contains("id_ed25519")));
}

#[cfg(unix)]
#[tokio::test]
async fn private_key_permissions_no_keys_found() {
    let dir = tempfile::tempdir().unwrap();
    // Empty ssh dir — no private key files.
    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "private_key_permissions");
    assert!(!matches.is_empty());
    assert!(matches.iter().any(|d| d.severity == Severity::Info));
}

// ---------------------------------------------------------------------------
// DefaultKeyExists
// ---------------------------------------------------------------------------

#[tokio::test]
async fn default_key_exists_when_present() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("id_ed25519"), "fake-key").unwrap();
    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "default_key_exists");
    assert!(!matches.is_empty());
    assert!(matches.iter().all(|d| d.severity == Severity::Ok));
    assert!(matches[0].message.contains("id_ed25519"));
}

#[tokio::test]
async fn default_key_missing() {
    let dir = tempfile::tempdir().unwrap();
    // No keys written.
    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "default_key_exists");
    assert!(!matches.is_empty());
    assert!(matches.iter().all(|d| d.severity == Severity::Warning));
}

// ---------------------------------------------------------------------------
// OwnerCheck (unix only)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[tokio::test]
async fn owner_check_passes_for_current_user() {
    let dir = tempfile::tempdir().unwrap();
    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "owner_check");
    assert!(!matches.is_empty());
    assert!(matches.iter().all(|d| d.severity == Severity::Ok));
}

// ---------------------------------------------------------------------------
// ConfigPermissionsCheck (unix only)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[tokio::test]
async fn config_permissions_ok_when_not_world_writable() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config");
    std::fs::write(&config_path, "").unwrap();
    std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o600)).unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "config_permissions");
    assert!(!matches.is_empty());
    assert!(matches.iter().all(|d| d.severity == Severity::Ok));
}

#[cfg(unix)]
#[tokio::test]
async fn config_permissions_error_when_world_writable() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config");
    std::fs::write(&config_path, "").unwrap();
    std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o666)).unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "config_permissions");
    assert!(!matches.is_empty());
    assert!(matches.iter().all(|d| d.severity == Severity::Error));
}

// ---------------------------------------------------------------------------
// AuthorizedKeysPermissionsCheck (unix only)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[tokio::test]
async fn authorized_keys_permissions_ok() {
    let dir = tempfile::tempdir().unwrap();
    let ak_path = dir.path().join("authorized_keys");
    std::fs::write(&ak_path, "").unwrap();
    std::fs::set_permissions(&ak_path, std::fs::Permissions::from_mode(0o600)).unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "authorized_keys_permissions");
    assert!(!matches.is_empty());
    assert!(matches.iter().all(|d| d.severity == Severity::Ok));
}

#[cfg(unix)]
#[tokio::test]
async fn authorized_keys_permissions_error_when_world_writable() {
    let dir = tempfile::tempdir().unwrap();
    let ak_path = dir.path().join("authorized_keys");
    std::fs::write(&ak_path, "").unwrap();
    std::fs::set_permissions(&ak_path, std::fs::Permissions::from_mode(0o666)).unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "authorized_keys_permissions");
    assert!(!matches.is_empty());
    assert!(matches.iter().all(|d| d.severity == Severity::Error));
}

#[tokio::test]
async fn authorized_keys_permissions_info_when_missing() {
    let dir = tempfile::tempdir().unwrap();
    // No authorized_keys file.
    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "authorized_keys_permissions");
    assert!(!matches.is_empty());
    assert!(matches.iter().all(|d| d.severity == Severity::Info));
}

// ---------------------------------------------------------------------------
// PublicKeyPairsCheck
// ---------------------------------------------------------------------------

#[tokio::test]
async fn public_key_pair_present() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("id_ed25519"), "fake-private-key").unwrap();
    std::fs::write(dir.path().join("id_ed25519.pub"), "fake-public-key").unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "public_key_pairs");
    assert!(matches.iter().any(|d| d.severity == Severity::Ok && d.message.contains("id_ed25519")));
}

#[tokio::test]
async fn public_key_pair_missing_pub() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("id_ed25519"), "fake-private-key").unwrap();
    // No .pub file.

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "public_key_pairs");
    assert!(matches.iter().any(|d| d.severity == Severity::Warning && d.message.contains("no matching public key")));
}

#[tokio::test]
async fn public_key_pairs_no_keys() {
    let dir = tempfile::tempdir().unwrap();
    // No id_* files at all.
    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "public_key_pairs");
    assert!(!matches.is_empty());
    assert!(matches.iter().all(|d| d.severity == Severity::Info));
}

// ---------------------------------------------------------------------------
// IdentityFileExistsCheck
// ---------------------------------------------------------------------------

#[tokio::test]
async fn identity_file_exists_present() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("my_key");
    std::fs::write(&key_path, "fake-key").unwrap();
    std::fs::write(
        dir.path().join("config"),
        format!("Host test\n    IdentityFile {}\n", key_path.display()),
    )
    .unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "identity_file_exists");
    assert!(matches.iter().any(|d| d.severity == Severity::Ok));
}

#[tokio::test]
async fn identity_file_exists_missing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("config"),
        "Host test\n    IdentityFile ~/.ssh/nonexistent_key\n",
    )
    .unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "identity_file_exists");
    assert!(matches.iter().any(|d| d.severity == Severity::Warning));
}

#[tokio::test]
async fn identity_file_exists_no_config() {
    let dir = tempfile::tempdir().unwrap();
    // No config file.
    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "identity_file_exists");
    assert!(!matches.is_empty());
    assert!(matches.iter().all(|d| d.severity == Severity::Info));
}

// ---------------------------------------------------------------------------
// DuplicateHostCheck
// ---------------------------------------------------------------------------

#[tokio::test]
async fn duplicate_host_detected() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("config"),
        "\
Host web
    HostName web1.example.com

Host web
    HostName web2.example.com
",
    )
    .unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "duplicate_host");
    assert!(matches.iter().any(|d| d.severity == Severity::Warning && d.message.contains("'web'")));
}

#[tokio::test]
async fn no_duplicates_ok() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("config"),
        "\
Host web
    HostName web.example.com

Host api
    HostName api.example.com
",
    )
    .unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "duplicate_host");
    assert!(matches.iter().all(|d| d.severity == Severity::Ok));
}

// ---------------------------------------------------------------------------
// HostStarPlacementCheck
// ---------------------------------------------------------------------------

#[tokio::test]
async fn host_star_before_specific_warns() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("config"),
        "\
Host *
    ServerAliveInterval 60

Host specific
    HostName specific.example.com
",
    )
    .unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "host_star_placement");
    assert!(matches.iter().any(|d| d.severity == Severity::Warning));
}

#[tokio::test]
async fn host_star_after_specific_ok() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("config"),
        "\
Host specific
    HostName specific.example.com

Host *
    ServerAliveInterval 60
",
    )
    .unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "host_star_placement");
    assert!(matches.iter().all(|d| d.severity == Severity::Ok));
}

// ---------------------------------------------------------------------------
// SshV1KeyCheck
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ssh_v1_key_not_present() {
    let dir = tempfile::tempdir().unwrap();
    // No "identity" file.
    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "ssh_v1_key");
    assert!(!matches.is_empty());
    assert!(matches.iter().all(|d| d.severity == Severity::Ok));
}

#[tokio::test]
async fn ssh_v1_key_detected() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("identity"), "fake-v1-key").unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "ssh_v1_key");
    assert!(matches.iter().any(|d| d.severity == Severity::Warning));
}

// ---------------------------------------------------------------------------
// AgentAvailable & KeygenAvailable (env-dependent, just ensure no crash)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn agent_available_returns_valid_diagnostic() {
    let dir = tempfile::tempdir().unwrap();
    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "agent_available");
    assert_eq!(matches.len(), 1);
    // Either Ok (if SSH_AUTH_SOCK is set and socket exists) or Warning.
    assert!(matches[0].severity == Severity::Ok || matches[0].severity == Severity::Warning);
}

#[tokio::test]
async fn keygen_available_returns_valid_diagnostic() {
    let dir = tempfile::tempdir().unwrap();
    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "keygen_available");
    assert_eq!(matches.len(), 1);
    // ssh-keygen should be available on macOS/Linux dev machines.
    assert!(matches[0].severity == Severity::Ok || matches[0].severity == Severity::Error);
}

// ---------------------------------------------------------------------------
// IdentityFilePubCheck
// ---------------------------------------------------------------------------

#[tokio::test]
async fn identity_file_pub_detected() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("config"),
        "Host test\n    IdentityFile ~/.ssh/id_ed25519.pub\n",
    )
    .unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "identity_file_pub");
    assert!(matches.iter().any(|d| d.severity == Severity::Warning));
}

#[tokio::test]
async fn identity_file_private_key_ok() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("config"),
        "Host test\n    IdentityFile ~/.ssh/id_ed25519\n",
    )
    .unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "identity_file_pub");
    assert!(matches.iter().all(|d| d.severity == Severity::Ok));
}

// ---------------------------------------------------------------------------
// IdentitiesOnlyCheck
// ---------------------------------------------------------------------------

#[tokio::test]
async fn identities_only_missing_warns() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("config"),
        "\
Host multi
    IdentityFile ~/.ssh/id_ed25519
    IdentityFile ~/.ssh/id_rsa
",
    )
    .unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "identities_only");
    assert!(matches.iter().any(|d| d.severity == Severity::Warning));
}

#[tokio::test]
async fn identities_only_present_ok() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("config"),
        "\
Host multi
    IdentityFile ~/.ssh/id_ed25519
    IdentityFile ~/.ssh/id_rsa
    IdentitiesOnly yes
",
    )
    .unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "identities_only");
    assert!(matches.iter().all(|d| d.severity == Severity::Ok));
}

#[tokio::test]
async fn identities_only_single_key_ok() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("config"),
        "Host single\n    IdentityFile ~/.ssh/id_ed25519\n",
    )
    .unwrap();

    let diags = run_checks_with_dir(dir.path()).await;
    let matches = find(&diags, "identities_only");
    assert!(matches.iter().all(|d| d.severity == Severity::Ok));
}
