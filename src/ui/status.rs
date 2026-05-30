use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Position, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph},
};

use crate::action::Action;
use crate::status::TorideStatus;
use crate::ui::responsive::{self, Viewport};
use crate::ui::theme::{self, Palette};

const KEY_BG: Color = Color::Rgb(32, 26, 50);
const HEADER_HEIGHT: u16 = 3;
const FOOTER_HEIGHT: u16 = 3;

pub struct StatusScreen {
    gradient_cache: Option<(Rect, Buffer)>,
    scroll: usize,
    status: Option<TorideStatus>,
}

impl Default for StatusScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusScreen {
    pub fn new() -> Self {
        Self {
            gradient_cache: None,
            scroll: 0,
            status: None,
        }
    }

    pub fn set_status(&mut self, status: TorideStatus) {
        self.status = Some(status);
    }

    pub fn invalidate_cache(&mut self) {
        self.gradient_cache = None;
    }

    pub fn handle_key(&self, code: ratatui::crossterm::event::KeyCode) -> Option<Action> {
        use ratatui::crossterm::event::KeyCode;
        match code {
            KeyCode::Char('b') | KeyCode::Esc => Some(Action::Back),
            KeyCode::Char('q') => Some(Action::Quit),
            _ => None,
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn clamp_scroll(&mut self, content_height: u16, viewport_height: u16) {
        let max_scroll = content_height.saturating_sub(viewport_height) as usize;
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
    }

    pub fn view(&mut self, frame: &mut Frame) {
        self.view_with_palette(frame, theme::CHARM);
    }

    fn view_with_palette(&mut self, frame: &mut Frame, p: Palette) {
        let area = frame.area();
        let viewport = Viewport::from_area(area);

        if responsive::render_too_small(frame, p) {
            return;
        }

        // Gradient background
        let buf = frame.buffer_mut();
        let needs_regen = !self
            .gradient_cache
            .as_ref()
            .is_some_and(|(cached_area, _)| *cached_area == area);
        if needs_regen {
            let mut gradient = Buffer::empty(area);
            render_gradient_bg(&mut gradient, area, p);
            copy_bg(&gradient, buf, area);
            self.gradient_cache = Some((area, gradient));
        } else if let Some((_, ref gradient)) = self.gradient_cache {
            copy_bg(gradient, buf, area);
        }

        // Adaptive center column
        let [_, center, _] = Layout::horizontal([
            Constraint::Fill(1),
            responsive::center_column(),
            Constraint::Fill(1),
        ])
        .flex(Flex::Center)
        .areas(area);

        // Vertical layout: header, content, footer
        let [header_area, content_area, footer_area] = Layout::vertical([
            Constraint::Length(HEADER_HEIGHT),
            Constraint::Fill(1),
            Constraint::Length(FOOTER_HEIGHT),
        ])
        .areas(center);

        // ── Header ──────────────────────────────────────────────────────
        let header_block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::new().fg(p.border))
            .style(Style::new().bg(p.bg_alt));
        let header_text = Line::from(Span::styled(
            "  System Status",
            Style::new().fg(p.accent).bold(),
        ));
        frame.render_widget(
            Paragraph::new(header_text)
                .block(header_block)
                .centered(),
            header_area,
        );

        // ── Content ─────────────────────────────────────────────────────
        let lines = if let Some(ref status) = self.status {
            build_status_lines(status, content_area.width as usize, viewport, p)
        } else {
            vec![Line::from(Span::styled(
                "  Collecting status...",
                Style::new().fg(p.text_dim),
            ))]
        };

        let content_block = Block::default()
            .borders(Borders::NONE)
            .padding(Padding::horizontal(1))
            .style(Style::new().bg(p.bg_inset));

        let inner = content_block.inner(content_area);
        let viewport_height = inner.height as usize;

        self.clamp_scroll(lines.len() as u16, viewport_height as u16);

        let visible: Vec<Line<'_>> = lines
            .into_iter()
            .skip(self.scroll)
            .take(viewport_height)
            .collect();

        frame.render_widget(content_block, content_area);
        frame.render_widget(Paragraph::new(visible), inner);

        // ── Footer ──────────────────────────────────────────────────────
        let footer_block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::new().fg(p.border))
            .style(Style::new().bg(p.bg_alt));

        let key_style = Style::new().fg(p.text).bg(KEY_BG);
        let lbl_style = Style::new().fg(p.text_muted);

        let footer_line = if viewport >= Viewport::Compact {
            let gap = Span::raw("     ");
            Line::from(vec![
                Span::styled(" j ", key_style),
                Span::raw(" "),
                Span::styled("down", lbl_style),
                gap.clone(),
                Span::styled(" k ", key_style),
                Span::raw(" "),
                Span::styled("up", lbl_style),
                gap.clone(),
                Span::styled(" b ", key_style),
                Span::raw(" "),
                Span::styled("back", lbl_style),
                gap.clone(),
                Span::styled(" q ", key_style),
                Span::raw(" "),
                Span::styled("quit", lbl_style),
            ])
        } else {
            Line::from(vec![
                Span::styled(" j ", key_style),
                Span::raw(" "),
                Span::styled(" k ", key_style),
                Span::raw(" "),
                Span::styled(" b ", key_style),
                Span::raw(" "),
                Span::styled(" q ", key_style),
            ])
        };

        frame.render_widget(
            Paragraph::new(footer_line).centered().block(footer_block),
            footer_area,
        );
    }
}

// ── Status line builder ──────────────────────────────────────────────────────

fn build_status_lines<'a>(
    status: &TorideStatus,
    _width: usize,
    viewport: Viewport,
    p: Palette,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line<'a>> = Vec::new();

    let section_style = Style::new().fg(p.accent).bold();
    let label_style = Style::new().fg(p.text);
    let value_style = Style::new().fg(p.text);
    let header_padding = if viewport >= Viewport::Compact { 2 } else { 1 };

    // ── System section ──────────────────────────────────────────────────
    lines.push(Line::from(Span::styled("  System", section_style)));
    for _ in 0..header_padding {
        lines.push(Line::from(""));
    }

    // Hostname
    lines.push(kv_line("Hostname", &status.system.hostname, label_style, value_style));

    // OS
    let os_name = status.system.os_info.name.as_deref().unwrap_or("Unknown");
    let os_version = status.system.os_info.version.as_deref().unwrap_or("unknown");
    let os_str = format!("{os_name} {os_version} ({})", status.system.os_info.arch);
    lines.push(kv_line("OS", &os_str, label_style, value_style));

    // CPU
    let (cpu_text, cpu_color) = match status.system.cpu_usage {
        Some(cpu) => {
            let color = percent_color(cpu, p);
            (format!("{cpu:.1}%"), color)
        }
        None => ("N/A".to_string(), p.text_dim),
    };
    lines.push(color_kv_line("CPU", &cpu_text, label_style, cpu_color));

    // Memory
    let mem_pct = status.system.memory.percentage;
    let mem_color = percent_color(mem_pct, p);
    let mem_text = format!(
        "{} / {} ({:.1}%)",
        format_bytes(status.system.memory.used_bytes),
        format_bytes(status.system.memory.total_bytes),
        mem_pct,
    );
    lines.push(color_kv_line("Memory", &mem_text, label_style, mem_color));

    // Disk
    let disk_pct = status.system.disk.percentage;
    let disk_color = percent_color(disk_pct, p);
    let disk_text = format!(
        "{} / {} ({:.1}%)",
        format_bytes(status.system.disk.used_bytes),
        format_bytes(status.system.disk.total_bytes),
        disk_pct,
    );
    lines.push(color_kv_line("Disk", &disk_text, label_style, disk_color));

    // Network
    let net_text = format!(
        "{} sent, {} received",
        format_bytes(status.system.network.bytes_transmitted),
        format_bytes(status.system.network.bytes_received),
    );
    lines.push(kv_line("Network", &net_text, label_style, value_style));

    // Uptime
    if let Some(secs) = status.system.uptime_secs {
        lines.push(kv_line("Uptime", &format_duration(secs), label_style, value_style));
    }

    // Load average
    if let Some(ref load) = status.system.load_average {
        let load_text = format!("{:.2} / {:.2} / {:.2}", load.one, load.five, load.fifteen);
        lines.push(kv_line("Load", &load_text, label_style, value_style));
    }

    // ── Daemon section ──────────────────────────────────────────────────
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Daemon", section_style)));
    for _ in 0..header_padding {
        lines.push(Line::from(""));
    }

    // Alive
    let (alive_text, alive_color) = if status.daemon.alive {
        ("alive", p.ok)
    } else {
        ("dead", p.err)
    };
    lines.push(color_kv_line(
        "Status",
        alive_text,
        label_style,
        alive_color,
    ));

    // PID
    if let Some(pid) = status.daemon.pid {
        lines.push(kv_line("PID", &pid.to_string(), label_style, value_style));
    }

    // Uptime
    if let Some(secs) = status.daemon.uptime_secs {
        lines.push(kv_line(
            "Uptime",
            &format!("{secs}s"),
            label_style,
            value_style,
        ));
    }

    // Restarts
    lines.push(kv_line(
        "Restarts",
        &status.daemon.restart_count.to_string(),
        label_style,
        value_style,
    ));

    // Socket
    let (socket_text, socket_color) = if status.daemon.stale_socket {
        ("stale", p.warn)
    } else {
        ("ok", p.ok)
    };
    lines.push(color_kv_line(
        "Socket",
        socket_text,
        label_style,
        socket_color,
    ));

    // ── SSH section ─────────────────────────────────────────────────────
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  SSH", section_style)));
    for _ in 0..header_padding {
        lines.push(Line::from(""));
    }

    // Mux master
    let (mux_text, mux_color) = if status.ssh.mux_master_alive {
        ("alive", p.ok)
    } else {
        ("dead", p.err)
    };
    lines.push(color_kv_line(
        "Mux master",
        mux_text,
        label_style,
        mux_color,
    ));

    // Control path
    let (ctl_text, ctl_color) = if status.ssh.control_path_valid {
        ("valid", p.ok)
    } else {
        ("invalid", p.err)
    };
    lines.push(color_kv_line(
        "Control path",
        ctl_text,
        label_style,
        ctl_color,
    ));

    // Config
    let (cfg_text, cfg_color) = if status.ssh.config_valid {
        ("ok", p.ok)
    } else {
        ("error", p.err)
    };
    lines.push(color_kv_line("Config", cfg_text, label_style, cfg_color));

    // Agent
    let (agent_text, agent_color) = if status.ssh.agent_running {
        ("running", p.ok)
    } else {
        ("stopped", p.warn)
    };
    lines.push(color_kv_line(
        "Agent",
        agent_text,
        label_style,
        agent_color,
    ));

    // Keys
    lines.push(kv_line(
        "Keys",
        &status.ssh.key_count.to_string(),
        label_style,
        value_style,
    ));

    // ── Capabilities section ────────────────────────────────────────────
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Capabilities", section_style)));
    for _ in 0..header_padding {
        lines.push(Line::from(""));
    }

    let caps = &status.capabilities;
    lines.push(yn_kv_line("CPU usage", caps.system.cpu_usage, label_style, p));
    lines.push(yn_kv_line(
        "Per-core CPU",
        caps.system.per_core_cpu,
        label_style,
        p,
    ));
    lines.push(yn_kv_line("Memory", caps.system.memory, label_style, p));
    lines.push(yn_kv_line("Swap", caps.system.swap, label_style, p));
    lines.push(yn_kv_line("Disk", caps.system.disk, label_style, p));
    lines.push(yn_kv_line("Network", caps.system.network, label_style, p));
    lines.push(yn_kv_line(
        "Load average",
        caps.system.load_average,
        label_style,
        p,
    ));
    lines.push(yn_kv_line("Uptime", caps.system.uptime, label_style, p));
    lines.push(yn_kv_line("Hostname", caps.system.hostname, label_style, p));
    lines.push(yn_kv_line("OS info", caps.system.os_info, label_style, p));
    lines.push(yn_kv_line("Sensors", caps.system.sensors, label_style, p));
    lines.push(yn_kv_line(
        "PID check",
        caps.daemon.pid_check,
        label_style,
        p,
    ));
    lines.push(yn_kv_line(
        "Uptime for PID",
        caps.daemon.uptime_for_pid,
        label_style,
        p,
    ));
    lines.push(yn_kv_line(
        "Stale socket",
        caps.daemon.stale_socket_detection,
        label_style,
        p,
    ));
    lines.push(yn_kv_line(
        "Mux check",
        caps.ssh.mux_check,
        label_style,
        p,
    ));
    lines.push(yn_kv_line(
        "Config validation",
        caps.ssh.config_validation,
        label_style,
        p,
    ));
    lines.push(yn_kv_line(
        "Agent check",
        caps.ssh.agent_check,
        label_style,
        p,
    ));
    lines.push(yn_kv_line(
        "Key counting",
        caps.ssh.key_counting,
        label_style,
        p,
    ));

    // ── Warnings ────────────────────────────────────────────────────────
    if !status.warnings.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("  Warnings", section_style)));
        for _ in 0..header_padding {
            lines.push(Line::from(""));
        }
        for w in &status.warnings {
            lines.push(Line::from(Span::styled(
                format!("    ! {w}"),
                Style::new().fg(p.warn),
            )));
        }
    }

    lines
}

// ── Line builders ────────────────────────────────────────────────────────────

fn kv_line<'a>(label: &str, value: &str, label_style: Style, value_style: Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("    {label}: "), label_style),
        Span::styled(value.to_string(), value_style),
    ])
}

fn color_kv_line<'a>(
    label: &str,
    value: &str,
    label_style: Style,
    value_color: Color,
) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("    {label}: "), label_style),
        Span::styled(value.to_string(), Style::new().fg(value_color)),
    ])
}

fn yn_kv_line<'a>(label: &str, value: bool, label_style: Style, p: Palette) -> Line<'a> {
    let (text, color) = if value {
        ("yes", p.ok)
    } else {
        ("no", p.text_dim)
    };
    color_kv_line(label, text, label_style, color)
}

fn percent_color(pct: f64, p: Palette) -> Color {
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

#[allow(clippy::cast_precision_loss)]
fn format_bytes(bytes: u64) -> String {
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

fn format_duration(secs: u64) -> String {
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

// ── Gradient background ──────────────────────────────────────────────────────

fn render_gradient_bg(buf: &mut Buffer, area: Rect, p: Palette) {
    let (cr, cg, cb) = rgb_components(p.bg);
    let er = (cr as f64 * 0.6) as u8;
    let eg = (cg as f64 * 0.6) as u8;
    let eb = (cb as f64 * 0.6) as u8;

    let cx = (area.left() + area.right()) / 2;
    let cy = (area.top() + area.bottom()) / 2;
    let max_dist = ((cx.saturating_sub(area.left()) as f64)
        .hypot(cy.saturating_sub(area.top()) as f64))
    .max(1.0);

    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let dx = (x as i32 - cx as i32).abs() as f64;
            let dy = (y as i32 - cy as i32).abs() as f64;
            let t = (dx.hypot(dy) / max_dist).min(1.0).powi(3);
            let r = lerp(cr as f64, er as f64, t) as u8;
            let g = lerp(cg as f64, eg as f64, t) as u8;
            let b = lerp(cb as f64, eb as f64, t) as u8;
            if let Some(cell) = buf.cell_mut(Position::new(x, y)) {
                cell.set_bg(Color::Rgb(r, g, b));
            }
        }
    }
}

fn rgb_components(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (0, 0, 0),
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a * (1.0 - t) + b * t
}

fn copy_bg(src: &Buffer, dst: &mut Buffer, area: Rect) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(s) = src.cell(Position::new(x, y))
                && let Some(d) = dst.cell_mut(Position::new(x, y))
            {
                d.set_bg(s.bg);
            }
        }
    }
}
