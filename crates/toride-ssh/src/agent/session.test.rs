use super::*;

#[test]
fn extract_host_from_cm_pattern() {
    let path = PathBuf::from("/home/user/.ssh/cm-root@server.example.com:22");
    assert_eq!(
        extract_host_from_socket_path(&path),
        "root@server.example.com"
    );
}

#[test]
fn extract_host_from_ctrl_pattern() {
    let path = PathBuf::from("/home/user/.ssh/ctrl-abc123def");
    assert_eq!(
        extract_host_from_socket_path(&path),
        "abc123def"
    );
}

#[test]
fn extract_host_from_ssh_tmp_pattern() {
    let path = PathBuf::from("/tmp/ssh-deploy@web01:22-mUXnBz");
    assert_eq!(
        extract_host_from_socket_path(&path),
        "deploy@web01"
    );
}

#[test]
fn extract_host_no_prefix() {
    let path = PathBuf::from("/home/user/.ssh/some-host:22");
    assert_eq!(
        extract_host_from_socket_path(&path),
        "some-host"
    );
}
