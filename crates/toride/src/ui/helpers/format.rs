use ratatui::style::{Color, Style};
use ratatui::text::Line;

use crate::ui::theme::Palette;

// ── Line builders ────────────────────────────────────────────────────────────

pub fn kv_line(label: &str, value: &str, label_style: Style, value_style: Style) -> Line<'static> {
    Line::from(vec![
        ratatui::text::Span::raw("    "),
        ratatui::text::Span::styled(label.to_string(), label_style),
        ratatui::text::Span::raw(": "),
        ratatui::text::Span::styled(value.to_string(), value_style),
    ])
}

pub fn color_kv_line(
    label: &str,
    value: &str,
    label_style: Style,
    value_color: Color,
) -> Line<'static> {
    Line::from(vec![
        ratatui::text::Span::raw("    "),
        ratatui::text::Span::styled(label.to_string(), label_style),
        ratatui::text::Span::raw(": "),
        ratatui::text::Span::styled(value.to_string(), Style::new().fg(value_color)),
    ])
}

pub fn yn_kv_line(label: &str, value: bool, label_style: Style, p: Palette) -> Line<'static> {
    let (text, color) = if value {
        ("yes", p.ok)
    } else {
        ("no", p.text_dim)
    };
    color_kv_line(label, text, label_style, color)
}

pub fn percent_color(pct: f64, p: Palette) -> Color {
    if pct >= 90.0 {
        p.err
    } else if pct >= 70.0 {
        p.warn
    } else {
        p.ok
    }
}

// ── Byte formatting ──────────────────────────────────────────────────────────

const KB: u64 = 1024;
const MB: u64 = KB * 1024;
const GB: u64 = MB * 1024;
const TB: u64 = GB * 1024;
const PB: u64 = TB * 1024;

#[expect(clippy::cast_precision_loss, reason = "u64→f64 for byte formatting display")]
pub fn format_bytes(bytes: u64) -> String {
    if bytes >= PB {
        format!("{:.1} PiB", bytes as f64 / PB as f64)
    } else if bytes >= TB {
        format!("{:.1} TiB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GiB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MiB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KiB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

pub fn format_duration(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if hours > 0 || !parts.is_empty() {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 || !parts.is_empty() {
        parts.push(format!("{minutes}m"));
    }
    parts.push(format!("{seconds}s"));
    parts.join(" ")
}
