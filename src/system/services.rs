pub fn has_systemd() -> bool {
    std::path::Path::new("/run/systemd/system").exists()
}

pub fn service_active(name: &str) -> bool {
    std::process::Command::new("systemctl")
        .args(["is-active", "--quiet", name])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
