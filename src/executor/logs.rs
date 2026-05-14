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

pub fn init_file_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let dir = log_dir();
    std::fs::create_dir_all(&dir).ok()?;

    let file_appender = tracing_appender::rolling::never(&dir, "setup.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("toride=info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    Some(guard)
}

pub fn log_action_jsonl(module_id: &str, action: &str, status: &str) {
    let dir = log_dir();
    let path = dir.join("actions.jsonl");
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let entry = serde_json::json!({
            "ts": chrono_now_rfc3339(),
            "module": module_id,
            "action": action,
            "status": status,
        });
        use std::io::Write;
        let _ = writeln!(file, "{}", entry);
    }
}

fn chrono_now_rfc3339() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
