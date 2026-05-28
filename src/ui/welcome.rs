use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Position, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};

use crate::action::Action;

const VERSION: &str = "0.4.1";
const EDITION: &str = "SINGLE-HOST";

// Charm palette — matches themes.js "charm" entry
const ACCENT: Color = Color::Rgb(255, 95, 203);     // #ff5fcb hot-pink
const ACCENT2: Color = Color::Rgb(162, 119, 255);   // #a277ff violet
const OK: Color = Color::Rgb(124, 227, 139);         // #7ce38b green
const TEXT: Color = Color::Rgb(244, 240, 255);       // #f4f0ff
const TEXT_DIM: Color = Color::Rgb(182, 168, 214);  // #b6a8d6
const TEXT_MUTED: Color = Color::Rgb(107, 95, 138); // #6b5f8a
const BORDER: Color = Color::Rgb(58, 46, 84);        // #3a2e54
const BG_INSET: Color = Color::Rgb(10, 10, 18);      // #0a0a12
const KEY_BG: Color = Color::Rgb(32, 26, 50);

// ANSI Shadow figlet — matches screens.jsx LOGO constant exactly
const LOGO: &[&str] = &[
    "████████╗ ██████╗ ██████╗ ██╗██████╗ ███████╗",
    "╚══██╔══╝██╔═══██╗██╔══██╗██║██╔══██╗██╔════╝",
    "   ██║   ██║   ██║██████╔╝██║██║  ██║█████╗  ",
    "   ██║   ██║   ██║██╔══██╗██║██║  ██║██╔══╝  ",
    "   ██║   ╚██████╔╝██║  ██║██║██████╔╝███████╗",
    "   ╚═╝    ╚═════╝ ╚═╝  ╚═╝╚═╝╚═════╝ ╚══════╝",
];

const STATUS_MESSAGES: &[(&str, &str)] = &[
    ("ok", "loaded /etc/toride/config.toml"),
    ("ok", "verifying SSH keypair (ed25519)"),
    ("ok", "apt available · 218 pkgs known"),
    ("ok", "docker engine 27.4.1 detected"),
    ("ok", "network: cloudflare 1.1.1.1 reachable"),
    ("ok", "ratatui v0.29.0 rendering · 60 fps"),
    ("--", "ready."),
];

pub struct WelcomeScreen;

impl WelcomeScreen {
    pub fn handle_key(&self, code: ratatui::crossterm::event::KeyCode) -> Option<Action> {
        use ratatui::crossterm::event::KeyCode;
        match code {
            KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
            KeyCode::Char('?') => Some(Action::Help),
            KeyCode::Enter | KeyCode::Char(' ') => Some(Action::Continue),
            _ => Some(Action::Continue),
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Radial gradient fills margins; panel uses BG_INSET override
        render_gradient_bg(frame.buffer_mut(), area);

        // Center column wide enough for logo (~45 cols) and panel
        let [_, center, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(72),
            Constraint::Fill(1),
        ])
        .flex(Flex::Center)
        .areas(area);

        // Layout: logo → spacer → version → prompt → spacer → panel → spacer → keys
        // panel = 2 borders + 2 v-padding + 7 messages = 11 rows
        let [
            _top,
            logo_area,
            _g1,
            version_area,
            prompt_area,
            _g2,
            panel_area,
            _g3,
            keys_area,
            _bottom,
        ] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(6),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(11),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(center);

        // ── Logo ──────────────────────────────────────────────────────────────
        let logo_lines: Vec<Line> = LOGO
            .iter()
            .map(|row| Line::from(Span::styled(*row, Style::new().fg(ACCENT).bold())))
            .collect();
        frame.render_widget(Paragraph::new(logo_lines).centered(), logo_area);

        // ── Version: "砦  ·  0.4.1  ·  SINGLE-HOST" ─────────────────────────
        let version_line = Line::from(vec![
            Span::styled("砦", Style::new().fg(ACCENT2).bold()),
            Span::styled("  ·  ", Style::new().fg(TEXT_MUTED)),
            Span::styled(VERSION, Style::new().fg(ACCENT2).bold()),
            Span::styled("  ·  ", Style::new().fg(TEXT_MUTED)),
            Span::styled(EDITION, Style::new().fg(ACCENT2).bold()),
        ]);
        frame.render_widget(Paragraph::new(version_line).centered(), version_area);

        // ── Prompt ────────────────────────────────────────────────────────────
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "Press any key, or click anywhere, to enter.",
                Style::new().fg(TEXT_DIM),
            )))
            .centered(),
            prompt_area,
        );

        // ── Status panel ──────────────────────────────────────────────────────
        let status_lines: Vec<Line> = STATUS_MESSAGES
            .iter()
            .map(|(tag, msg)| {
                let tag_color = if *tag == "ok" { OK } else { ACCENT };
                Line::from(vec![
                    Span::styled("[", Style::new().fg(tag_color).bold()),
                    Span::styled(*tag, Style::new().fg(tag_color).bold()),
                    Span::styled("]", Style::new().fg(tag_color).bold()),
                    Span::raw(" "),
                    Span::styled(*msg, Style::new().fg(TEXT)),
                ])
            })
            .collect();

        let panel_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(BORDER))
            .padding(Padding::new(2, 2, 1, 1))
            .style(Style::new().bg(BG_INSET));

        frame.render_widget(Paragraph::new(status_lines).block(panel_block), panel_area);

        // ── Keybindings — styled as keyboard badges ───────────────────────────
        let key_style = Style::new().fg(TEXT).bg(KEY_BG);
        let lbl_style = Style::new().fg(TEXT_MUTED);
        let gap = Span::raw("     ");
        let keys_line = Line::from(vec![
            Span::styled(" ↵ ", key_style),
            Span::raw(" "),
            Span::styled("continue", lbl_style),
            gap.clone(),
            Span::styled(" ? ", key_style),
            Span::raw(" "),
            Span::styled("help", lbl_style),
            gap.clone(),
            Span::styled(" q ", key_style),
            Span::raw(" "),
            Span::styled("quit", lbl_style),
        ]);
        frame.render_widget(Paragraph::new(keys_line).centered(), keys_area);
    }
}

// Radial gradient: center (#1a1628) → edges (#0e0c16)
fn render_gradient_bg(buf: &mut Buffer, area: Rect) {
    let cx = (area.left() + area.right()) / 2;
    let cy = (area.top() + area.bottom()) / 2;
    let max_dist = ((cx.saturating_sub(area.left()) as f64)
        .hypot(cy.saturating_sub(area.top()) as f64))
    .max(1.0);

    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let dx = (x as i32 - cx as i32).abs() as f64;
            let dy = (y as i32 - cy as i32).abs() as f64;
            let t = (dx.hypot(dy) / max_dist).min(1.0);
            let r = lerp(26.0, 14.0, t) as u8;
            let g = lerp(22.0, 12.0, t) as u8;
            let b = lerp(40.0, 22.0, t) as u8;
            if let Some(cell) = buf.cell_mut(Position::new(x, y)) {
                cell.set_bg(Color::Rgb(r, g, b));
            }
        }
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a * (1.0 - t) + b * t
}
