use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Position, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::action::Action;
use crate::ui::responsive::{self, Viewport};
use crate::ui::theme::{self, Palette};

const KEY_BG: Color = Color::Rgb(32, 26, 50);

pub struct HelpScreen {
    gradient_cache: Option<(Rect, Buffer)>,
}

impl Default for HelpScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpScreen {
    pub fn new() -> Self {
        Self {
            gradient_cache: None,
        }
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

        // Vertical layout
        let [
            _top,
            title_area,
            _g1,
            bindings_area,
            _g2,
            keys_area,
            _bottom,
        ] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(7),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(center);

        // ── Title ───────────────────────────────────────────────────────
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "Help",
                Style::new().fg(p.accent).bold(),
            )))
            .centered(),
            title_area,
        );

        // ── Keybindings ─────────────────────────────────────────────────
        let key_style = Style::new().fg(p.text).bg(KEY_BG);
        let lbl_style = Style::new().fg(p.text);

        let entries: Vec<Line<'_>> = if viewport >= Viewport::Compact {
            vec![
                keybinding_line("Enter", "Show system status", key_style, lbl_style),
                keybinding_line("?", "Show this help", key_style, lbl_style),
                keybinding_line("q", "Quit", key_style, lbl_style),
                keybinding_line("j / Down", "Scroll down", key_style, lbl_style),
                keybinding_line("k / Up", "Scroll up", key_style, lbl_style),
                keybinding_line("b / Esc", "Back to welcome", key_style, lbl_style),
            ]
        } else {
            vec![
                keybinding_line("Enter", "Status", key_style, lbl_style),
                keybinding_line("?", "Help", key_style, lbl_style),
                keybinding_line("q", "Quit", key_style, lbl_style),
                keybinding_line("j/k", "Scroll", key_style, lbl_style),
                keybinding_line("b", "Back", key_style, lbl_style),
            ]
        };

        frame.render_widget(Paragraph::new(entries).centered(), bindings_area);

        // ── Back hint ───────────────────────────────────────────────────
        let back_line = Line::from(vec![
            Span::styled(" b ", key_style),
            Span::raw(" "),
            Span::styled("back", Style::new().fg(p.text_muted)),
        ]);
        frame.render_widget(Paragraph::new(back_line).centered(), keys_area);
    }
}

fn keybinding_line<'a>(
    key: &str,
    desc: &str,
    key_style: Style,
    lbl_style: Style,
) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!(" {key} "), key_style),
        Span::raw("  "),
        Span::styled(desc.to_string(), lbl_style),
    ])
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
