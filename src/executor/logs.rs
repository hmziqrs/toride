use std::path::PathBuf;

pub fn log_dir() -> PathBuf {
    if nix::unistd::geteuid().is_root() {
        PathBuf::from("/var/log/toride")
    } else {
        dirs::state_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("toride")
            .join("logs")
    }
}
