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
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let days = secs / 86400;
    // 1970-01-01 + days -> calculate date
    let (year, month, day) = days_to_date(days);
    let time_of_day = secs % 86400;
    let hours = (time_of_day / 3600) as u8;
    let minutes = ((time_of_day % 3600) / 60) as u8;
    let seconds = (time_of_day % 60) as u8;
    let subsec = dur.subsec_millis();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year, month, day, hours, minutes, seconds, subsec
    )
}

fn days_to_date(mut days: u64) -> (u64, u8, u8) {
    let mut y = 1970u64;
    loop {
        let dy = if is_leap(y) { 366 } else { 365 };
        if days < dy { break; }
        days -= dy;
        y += 1;
    }
    let leap = is_leap(y);
    let md = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut m = 0u8;
    for &d_in_m in &md {
        m += 1;
        if days < d_in_m { return (y, m, (days + 1) as u8); }
        days -= d_in_m;
    }
    (y, 12, 31)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
