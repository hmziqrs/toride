#[derive(Debug, Clone)]
pub struct OsInfo {
    pub id: String,
    pub id_like: String,
    pub name: String,
    pub version: String,
    pub codename: String,
}

pub fn detect() -> Option<OsInfo> {
    let content = std::fs::read_to_string("/etc/os-release").ok()?;
    Some(OsInfo {
        id: parse_field(&content, "ID"),
        id_like: parse_field(&content, "ID_LIKE"),
        name: parse_field(&content, "PRETTY_NAME"),
        version: parse_field(&content, "VERSION_ID"),
        codename: parse_field(&content, "VERSION_CODENAME"),
    })
}

fn parse_field(content: &str, field: &str) -> String {
    let prefix = format!("{}=", field);
    content.lines()
        .find(|l| l.starts_with(&prefix))
        .and_then(|l| l.strip_prefix(&prefix))
        .map(|v| v.trim_matches('"').to_string())
        .unwrap_or_default()
}

pub fn is_supported(os: &OsInfo) -> bool {
    matches!(os.id.as_str(), "debian" | "ubuntu")
}
