use crossterm::event::{KeyCode, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph},
};
use rattles::Rattler;

use crate::action::Action;
use crate::status::{
    Capabilities, DaemonStatus, SshStatus, StatusError, SystemStatus, TorideStatus,
};
use crate::ui::helpers::{
    color_kv_line, format_bytes, format_duration, kv_line, percent_color, yn_kv_line,
};
use crate::ui::responsive::{self, Viewport};
use crate::ui::screens::AppScreen;
use crate::ui::theme::Palette;
use crate::ui::widgets::gradient::GradientCache;

const HEADER_HEIGHT: u16 = 3;
const FOOTER_HEIGHT: u16 = 3;

pub struct StatusScreen {
    gradient_cache: GradientCache,
    cached_lines: Option<Vec<Line<'static>>>,
    scroll: usize,
    status: Option<TorideStatus>,
    spinner: Rattler<rattles::presets::braille::Dots>,
}

impl Default for StatusScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl AppScreen for StatusScreen {
    fn handle_key(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Char('j') | KeyCode::Down => Some(Action::ScrollDown),
            KeyCode::Char('k') | KeyCode::Up => Some(Action::ScrollUp),
            KeyCode::Char('b') | KeyCode::Esc => Some(Action::Back),
            KeyCode::Char('q') => Some(Action::Quit),
            _ => None,
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> Option<Action> {
        match mouse.kind {
            MouseEventKind::ScrollDown => Some(Action::ScrollDown),
            MouseEventKind::ScrollUp => Some(Action::ScrollUp),
            _ => None,
        }
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::ScrollDown => self.scroll_down(),
            Action::ScrollUp => self.scroll_up(),
            _ => {}
        }
    }

    fn view(&mut self, frame: &mut Frame, palette: Palette) {
        self.render(frame, palette, false);
    }

    fn view_foreground(&mut self, frame: &mut Frame, palette: Palette) {
        self.render(frame, palette, true);
    }

    fn invalidate_cache(&mut self) {
        self.gradient_cache.invalidate();
    }

    fn needs_animation(&self) -> bool {
        self.status.is_none()
    }
}

impl StatusScreen {
    pub fn new() -> Self {
        Self {
            gradient_cache: GradientCache::new(),
            cached_lines: None,
            scroll: 0,
            status: None,
            spinner: rattles::presets::braille::dots(),
        }
    }

    pub fn set_status(&mut self, status: TorideStatus) {
        self.status = Some(status);
        self.cached_lines = None;
    }

    fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    fn clamp_scroll(&mut self, content_height: u16, viewport_height: u16) {
        let max_scroll = content_height.saturating_sub(viewport_height) as usize;
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
    }

    fn render_loading(
        &mut self,
        frame: &mut Frame,
        content_area: ratatui::layout::Rect,
        p: Palette,
        footer_area: ratatui::layout::Rect,
    ) {
        let spinner_frame = self.spinner.current_frame();
        let loading_line = Line::from(vec![
            Span::styled(format!("  {spinner_frame} "), Style::new().fg(p.accent)),
            Span::styled("Collecting status...", Style::new().fg(p.text_dim)),
        ]);

        let content_block = Block::default()
            .padding(Padding::horizontal(1))
            .style(Style::new().bg(p.bg_inset));
        frame.render_widget(
            Paragraph::new(loading_line).block(content_block),
            content_area,
        );

        // Footer
        let footer_block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::new().fg(p.border))
            .style(Style::new().bg(p.bg_alt));
        frame.render_widget(
            Paragraph::new(Line::from(""))
                .centered()
                .block(footer_block),
            footer_area,
        );
    }

    fn render(&mut self, frame: &mut Frame, p: Palette, skip_bg: bool) {
        let area = frame.area();
        let viewport = Viewport::from_area(area);

        if responsive::render_too_small(frame, p) {
            return;
        }

        // Gradient background
        if !skip_bg {
            let buf = frame.buffer_mut();
            self.gradient_cache.render_or_copy(buf, area, p);
        }

        // Adaptive center column
        let center = responsive::center_area(area);

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
            Paragraph::new(header_text).block(header_block).centered(),
            header_area,
        );

        // ── Content ─────────────────────────────────────────────────────
        if self.cached_lines.is_none() {
            if let Some(ref status) = self.status {
                self.cached_lines = Some(build_status_lines(status, viewport, p));
            } else {
                // Loading — don't cache so the spinner frame advances each tick
                return self.render_loading(frame, content_area, p, footer_area);
            }
        }

        let content_block = Block::default()
            .padding(Padding::horizontal(1))
            .style(Style::new().bg(p.bg_inset));
        let inner = content_block.inner(content_area);
        let viewport_height = inner.height as usize;

        // Compute visible window — extract count before borrowing lines
        #[expect(
            clippy::cast_possible_truncation,
            reason = "line count fits in u16 for TUI display"
        )]
        let line_count = self
            .cached_lines
            .as_ref()
            .expect("cached_lines populated above")
            .len() as u16;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "viewport height fits in u16"
        )]
        self.clamp_scroll(line_count, viewport_height as u16);

        let visible: Vec<Line<'_>> = self
            .cached_lines
            .as_ref()
            .expect("cached_lines populated above")
            .iter()
            .skip(self.scroll)
            .take(viewport_height)
            .cloned()
            .collect();
        frame.render_widget(Paragraph::new(visible).block(content_block), content_area);

        // ── Footer ──────────────────────────────────────────────────────
        let footer_block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::new().fg(p.border))
            .style(Style::new().bg(p.bg_alt));

        let key_style = p.key_style();
        let lbl_style = p.label_style();

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

fn build_status_lines(status: &TorideStatus, viewport: Viewport, p: Palette) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.extend(build_system_section(&status.system, viewport, p));
    lines.extend(build_daemon_section(&status.daemon, viewport, p));
    lines.extend(build_ssh_section(&status.ssh, viewport, p));
    lines.extend(build_capabilities_section(
        &status.capabilities,
        viewport,
        p,
    ));
    lines.extend(build_warnings_section(&status.warnings, viewport, p));
    lines
}

fn build_system_section(
    system: &SystemStatus,
    viewport: Viewport,
    p: Palette,
) -> Vec<Line<'static>> {
    let section_style = Style::new().fg(p.accent).bold();
    let label_style = Style::new().fg(p.text_dim);
    let value_style = Style::new().fg(p.text);
    let header_padding = if viewport >= Viewport::Compact { 2 } else { 1 };

    let mut lines: Vec<Line<'static>> = Vec::new();

    lines.push(Line::from(Span::styled("  System", section_style)));
    for _ in 0..header_padding {
        lines.push(Line::from(""));
    }

    // Hostname
    lines.push(kv_line(
        "Hostname",
        &system.hostname,
        label_style,
        value_style,
    ));

    // OS
    let os_name = system.os_info.name.as_deref().unwrap_or("Unknown");
    let os_version = system.os_info.version.as_deref().unwrap_or("unknown");
    let os_str = format!("{os_name} {os_version} ({})", system.os_info.arch);
    lines.push(kv_line("OS", &os_str, label_style, value_style));

    // CPU
    let (cpu_text, cpu_color) = match system.cpu_usage {
        Some(cpu) => {
            let color = percent_color(cpu, p);
            (format!("{cpu:.1}%"), color)
        }
        None => ("N/A".to_string(), p.text_dim),
    };
    lines.push(color_kv_line("CPU", &cpu_text, label_style, cpu_color));

    // Memory
    let mem_pct = system.memory.percentage;
    let mem_color = percent_color(mem_pct, p);
    let mem_text = format!(
        "{} / {} ({:.1}%)",
        format_bytes(system.memory.used_bytes),
        format_bytes(system.memory.total_bytes),
        mem_pct,
    );
    lines.push(color_kv_line("Memory", &mem_text, label_style, mem_color));

    // Disk
    let disk_pct = system.disk.percentage;
    let disk_color = percent_color(disk_pct, p);
    let disk_text = format!(
        "{} / {} ({:.1}%)",
        format_bytes(system.disk.used_bytes),
        format_bytes(system.disk.total_bytes),
        disk_pct,
    );
    lines.push(color_kv_line("Disk", &disk_text, label_style, disk_color));

    // Network
    let net_text = format!(
        "{} sent, {} received",
        format_bytes(system.network.bytes_transmitted),
        format_bytes(system.network.bytes_received),
    );
    lines.push(kv_line("Network", &net_text, label_style, value_style));

    // Uptime
    if let Some(secs) = system.uptime_secs {
        lines.push(kv_line(
            "Uptime",
            &format_duration(secs),
            label_style,
            value_style,
        ));
    }

    // Load average
    if let Some(ref load) = system.load_average {
        let load_text = format!("{:.2} / {:.2} / {:.2}", load.one, load.five, load.fifteen);
        lines.push(kv_line("Load", &load_text, label_style, value_style));
    }

    lines
}

fn build_daemon_section(
    daemon: &DaemonStatus,
    viewport: Viewport,
    p: Palette,
) -> Vec<Line<'static>> {
    let section_style = Style::new().fg(p.accent).bold();
    let label_style = Style::new().fg(p.text_dim);
    let value_style = Style::new().fg(p.text);
    let header_padding = if viewport >= Viewport::Compact { 2 } else { 1 };

    let mut lines: Vec<Line<'static>> = Vec::new();

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Daemon", section_style)));
    for _ in 0..header_padding {
        lines.push(Line::from(""));
    }

    // Alive
    let (alive_text, alive_color) = if daemon.alive {
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
    if let Some(pid) = daemon.pid {
        lines.push(kv_line("PID", &pid.to_string(), label_style, value_style));
    }

    // Uptime
    if let Some(secs) = daemon.uptime_secs {
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
        &daemon.restart_count.to_string(),
        label_style,
        value_style,
    ));

    // Socket
    let (socket_text, socket_color) = if daemon.stale_socket {
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

    lines
}

fn build_ssh_section(ssh: &SshStatus, viewport: Viewport, p: Palette) -> Vec<Line<'static>> {
    let section_style = Style::new().fg(p.accent).bold();
    let label_style = Style::new().fg(p.text_dim);
    let value_style = Style::new().fg(p.text);
    let header_padding = if viewport >= Viewport::Compact { 2 } else { 1 };

    let mut lines: Vec<Line<'static>> = Vec::new();

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  SSH", section_style)));
    for _ in 0..header_padding {
        lines.push(Line::from(""));
    }

    // Mux master
    let (mux_text, mux_color) = if ssh.mux_master_alive {
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
    let (ctl_text, ctl_color) = if ssh.control_path_valid {
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
    let (cfg_text, cfg_color) = if ssh.config_valid {
        ("ok", p.ok)
    } else {
        ("error", p.err)
    };
    lines.push(color_kv_line("Config", cfg_text, label_style, cfg_color));

    // Agent
    let (agent_text, agent_color) = if ssh.agent_running {
        ("running", p.ok)
    } else {
        ("stopped", p.warn)
    };
    lines.push(color_kv_line("Agent", agent_text, label_style, agent_color));

    // Keys
    lines.push(kv_line(
        "Keys",
        &ssh.key_count.to_string(),
        label_style,
        value_style,
    ));

    lines
}

fn build_capabilities_section(
    caps: &Capabilities,
    viewport: Viewport,
    p: Palette,
) -> Vec<Line<'static>> {
    let section_style = Style::new().fg(p.accent).bold();
    let label_style = Style::new().fg(p.text_dim);
    let header_padding = if viewport >= Viewport::Compact { 2 } else { 1 };

    let mut lines: Vec<Line<'static>> = Vec::new();

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Capabilities", section_style)));
    for _ in 0..header_padding {
        lines.push(Line::from(""));
    }

    lines.push(yn_kv_line(
        "CPU usage",
        caps.system.cpu_usage,
        label_style,
        p,
    ));
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
    lines.push(yn_kv_line("Mux check", caps.ssh.mux_check, label_style, p));
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

    lines
}

fn build_warnings_section(
    warnings: &[StatusError],
    viewport: Viewport,
    p: Palette,
) -> Vec<Line<'static>> {
    let section_style = Style::new().fg(p.accent).bold();
    let header_padding = if viewport >= Viewport::Compact { 2 } else { 1 };

    let mut lines: Vec<Line<'static>> = Vec::new();

    if warnings.is_empty() {
        return lines;
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Warnings", section_style)));
    for _ in 0..header_padding {
        lines.push(Line::from(""));
    }
    for w in warnings {
        lines.push(Line::from(Span::styled(
            format!("    ! {w}"),
            Style::new().fg(p.warn),
        )));
    }

    lines
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;

    use super::StatusScreen;
    use crate::action::Action;
    use crate::ui::screens::AppScreen;

    #[test]
    fn new_creates_with_default_state() {
        let screen = StatusScreen::new();
        assert_eq!(screen.scroll, 0, "scroll should start at 0");
        assert!(screen.status.is_none(), "status should be None");
        assert!(screen.cached_lines.is_none(), "cached_lines should be None");
    }

    #[test]
    fn handle_key_returns_back_for_b() {
        let mut screen = StatusScreen::new();
        assert_eq!(screen.handle_key(KeyCode::Char('b')), Some(Action::Back));
    }

    #[test]
    fn handle_key_returns_back_for_esc() {
        let mut screen = StatusScreen::new();
        assert_eq!(screen.handle_key(KeyCode::Esc), Some(Action::Back));
    }

    #[test]
    fn handle_key_returns_quit_for_q() {
        let mut screen = StatusScreen::new();
        assert_eq!(screen.handle_key(KeyCode::Char('q')), Some(Action::Quit));
    }

    #[test]
    fn handle_key_returns_scroll_down_for_j() {
        let mut screen = StatusScreen::new();
        assert_eq!(
            screen.handle_key(KeyCode::Char('j')),
            Some(Action::ScrollDown)
        );
    }

    #[test]
    fn handle_key_returns_scroll_down_for_down() {
        let mut screen = StatusScreen::new();
        assert_eq!(screen.handle_key(KeyCode::Down), Some(Action::ScrollDown));
    }

    #[test]
    fn handle_key_returns_scroll_up_for_k() {
        let mut screen = StatusScreen::new();
        assert_eq!(
            screen.handle_key(KeyCode::Char('k')),
            Some(Action::ScrollUp)
        );
    }

    #[test]
    fn handle_key_returns_scroll_up_for_up() {
        let mut screen = StatusScreen::new();
        assert_eq!(screen.handle_key(KeyCode::Up), Some(Action::ScrollUp));
    }

    #[test]
    fn handle_action_scroll_down_calls_scroll_down() {
        let mut screen = StatusScreen::new();
        assert_eq!(screen.scroll, 0);
        screen.handle_action(Action::ScrollDown);
        assert_eq!(screen.scroll, 1, "scroll should increment after ScrollDown");
    }

    #[test]
    fn needs_animation_true_when_status_is_none() {
        let screen = StatusScreen::new();
        assert!(
            screen.needs_animation(),
            "needs_animation should be true when status is None"
        );
    }

    #[test]
    fn needs_animation_false_after_set_status() {
        let mut screen = StatusScreen::new();
        let status = crate::status::TorideStatus::collect();
        screen.set_status(status);
        assert!(
            !screen.needs_animation(),
            "needs_animation should be false after set_status"
        );
    }
}
