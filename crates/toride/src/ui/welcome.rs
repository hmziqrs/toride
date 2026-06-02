use std::time::Instant;

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Position, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use tachyonfx::{Interpolatable, color_from_hsl, color_to_hsl};

use crate::action::Action;
use crate::ui::responsive::{self, Viewport};
use crate::ui::theme::{self, Palette};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const EDITION: &str = "SINGLE-HOST";

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

pub struct WelcomeScreen {
    gradient_cache: Option<(Rect, Buffer)>,
    anim_start: Instant,
    color_cycle: Vec<Color>,
}

impl Default for WelcomeScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl WelcomeScreen {
    #[must_use]
    pub fn new() -> Self {
        Self {
            gradient_cache: None,
            anim_start: Instant::now(),
            color_cycle: build_color_cycle(theme::CHARM.accent),
        }
    }

    pub fn invalidate_cache(&mut self) {
        self.gradient_cache = None;
    }

    #[must_use]
    pub fn handle_key(&self, code: ratatui::crossterm::event::KeyCode) -> Option<Action> {
        use ratatui::crossterm::event::KeyCode;
        match code {
            KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
            KeyCode::Char('?') => Some(Action::Help),
            KeyCode::Enter | KeyCode::Char(' ') => Some(Action::Continue),
            _ => None,
        }
    }

    pub fn view(&mut self, frame: &mut Frame) {
        self.view_with_palette(frame, theme::CHARM);
    }

    fn view_with_palette(&mut self, frame: &mut Frame, p: Palette) {
        let area = frame.area();
        let viewport = Viewport::from_area(area);

        // Fallback for tiny terminals
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
            logo_area,
            _g1,
            version_area,
            prompt_area,
            _g2,
            keys_area,
            _bottom,
        ] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(6),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(center);

        // ── Animated border ───────────────────────────────────────────────
        let border_rect = content_border_rect(logo_area, keys_area, area);
        let elapsed = self.anim_start.elapsed().as_secs_f32();
        draw_animated_border(buf, border_rect, &self.color_cycle, elapsed);

        // ── Logo ──────────────────────────────────────────────────────────
        let logo_style = Style::new().fg(p.accent).bold();
        let logo_lines = responsive::truncate_logo(LOGO, center.width, logo_style);
        frame.render_widget(Paragraph::new(logo_lines).centered(), logo_area);

        // ── Version ───────────────────────────────────────────────────────
        let version_line = Line::from(vec![
            Span::styled("砦", Style::new().fg(p.accent2).bold()),
            Span::styled("  ·  ", Style::new().fg(p.text_muted)),
            Span::styled(VERSION, Style::new().fg(p.accent2).bold()),
            Span::styled("  ·  ", Style::new().fg(p.text_muted)),
            Span::styled(EDITION, Style::new().fg(p.accent2).bold()),
        ]);
        frame.render_widget(Paragraph::new(version_line).centered(), version_area);

        // ── Prompt ────────────────────────────────────────────────────────
        let prompt_text = if viewport >= Viewport::Compact {
            "Press any key, or click anywhere, to enter."
        } else {
            "Press any key to enter."
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                prompt_text,
                Style::new().fg(p.text_dim),
            )))
            .centered(),
            prompt_area,
        );

        // ── Keybindings ───────────────────────────────────────────────────
        let key_style = Style::new().fg(p.text).bg(KEY_BG);
        let lbl_style = Style::new().fg(p.text_muted);

        let keys_line = if viewport >= Viewport::Compact {
            let gap = Span::raw("     ");
            Line::from(vec![
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
            ])
        } else {
            // Minimal — badges only, no labels
            Line::from(vec![
                Span::styled(" ↵ ", key_style),
                Span::raw(" "),
                Span::styled(" ? ", key_style),
                Span::raw(" "),
                Span::styled(" q ", key_style),
            ])
        };
        frame.render_widget(Paragraph::new(keys_line).centered(), keys_area);
    }
}

// ── Gradient background ──────────────────────────────────────────────────────

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    reason = "color math: f64->u8 truncation is intentional for RGB blending"
)]
fn render_gradient_bg(buf: &mut Buffer, area: Rect, p: Palette) {
    let (cr, cg, cb) = rgb_components(p.bg);
    let er = (f64::from(cr) * 0.6) as u8;
    let eg = (f64::from(cg) * 0.6) as u8;
    let eb = (f64::from(cb) * 0.6) as u8;

    let cx = u16::midpoint(area.left(), area.right());
    let cy = u16::midpoint(area.top(), area.bottom());
    let max_dist = ((f64::from(cx.saturating_sub(area.left())))
        .hypot(f64::from(cy.saturating_sub(area.top()))))
    .max(1.0);

    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let dx = f64::from(i32::from(x).abs_diff(i32::from(cx)));
            let dy = f64::from(i32::from(y).abs_diff(i32::from(cy)));
            let t = (dx.hypot(dy) / max_dist).min(1.0).powi(3);
            let r = lerp(f64::from(cr), f64::from(er), t) as u8;
            let g = lerp(f64::from(cg), f64::from(eg), t) as u8;
            let b = lerp(f64::from(cb), f64::from(eb), t) as u8;
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

// ── Animated border ───────────────────────────────────────────────────────────

/// Compute the border rect as the union of content areas expanded by 2 cells
/// of padding, clamped to the frame area.
fn content_border_rect(logo_area: Rect, keys_area: Rect, frame_area: Rect) -> Rect {
    let pad = 2u16;
    let x = logo_area.x.saturating_sub(pad).max(frame_area.x);
    let y = logo_area.y.saturating_sub(pad).max(frame_area.y);
    let right = (keys_area.right() + pad).min(frame_area.right());
    let bottom = (keys_area.bottom() + pad).min(frame_area.bottom());
    Rect {
        x,
        y,
        width: right.saturating_sub(x),
        height: bottom.saturating_sub(y),
    }
}

/// Build a seamless looping color gradient from a base color using HSL manipulation.
/// Ported from exabind's `select_category_color_cycle()`, with a final wrap-around
/// segment that interpolates back to the base color for smooth looping at corners.
fn build_color_cycle(base_color: Color) -> Vec<Color> {
    let (h, s, l) = color_to_hsl(&base_color);

    let color_l = color_from_hsl(h, s, 80.0);
    let color_d = color_from_hsl(h, s, 40.0);
    let color_hue_neg = color_from_hsl((h - 25.0).rem_euclid(360.0), s, (l + 10.0).min(100.0));
    let color_sat_neg = color_from_hsl(h, (s - 20.0).max(0.0), (l + 10.0).min(100.0));
    let color_hue_pos = color_from_hsl((h + 25.0).rem_euclid(360.0), s, (l + 10.0).min(100.0));
    let color_sat_pos = color_from_hsl(h, (s + 20.0).min(100.0), (l + 10.0).min(100.0));

    let keyframes: &[(usize, Color)] = &[
        (4, color_d),
        (2, color_l),
        (4, color_hue_neg),
        (7, color_sat_neg),
        (7, color_hue_pos),
        (7, color_sat_pos),
    ];

    let mut colors = vec![base_color];
    let mut prev = base_color;
    for &(steps, target) in keyframes {
        for i in 1..=steps {
            colors.push(prev.lerp(&target, i as f32 / steps as f32));
        }
        colors.push(target);
        prev = target;
    }

    // Wrap-around: interpolate from last keyframe back to base color
    // so the cycle loops seamlessly at the join point (top-left corner).
    let wrap_steps = 7;
    for i in 1..wrap_steps {
        colors.push(prev.lerp(&base_color, i as f32 / wrap_steps as f32));
    }

    colors
}

/// Draw an animated color-cycling border around `border_rect`.
///
/// Walks the perimeter clockwise (top→right→bottom→left), drawing box-drawing
/// characters with foreground colors that cycle over time, producing a flowing
/// rainbow effect at ~12 cells/second.
fn draw_animated_border(
    buf: &mut Buffer,
    border_rect: Rect,
    color_cycle: &[Color],
    elapsed_secs: f32,
) {
    if border_rect.width < 3 || border_rect.height < 3 {
        return;
    }

    let idx = (elapsed_secs * 12.0) as usize;
    let cycle_len = color_cycle.len();
    let mut perimeter_idx = 0usize;

    let color_at = |pidx: usize| -> Color { color_cycle[(idx + pidx) % cycle_len] };

    let set_cell = |buf: &mut Buffer, x: u16, y: u16, ch: char, pidx: usize| {
        if let Some(cell) = buf.cell_mut(Position::new(x, y)) {
            cell.set_char(ch);
            cell.set_fg(color_at(pidx));
        }
    };

    let x0 = border_rect.x;
    let y0 = border_rect.y;
    let x1 = border_rect.right() - 1;
    let y1 = border_rect.bottom() - 1;

    // Top edge: left → right
    set_cell(buf, x0, y0, '┌', perimeter_idx);
    perimeter_idx += 1;
    for x in (x0 + 1)..x1 {
        set_cell(buf, x, y0, '─', perimeter_idx);
        perimeter_idx += 1;
    }
    set_cell(buf, x1, y0, '┐', perimeter_idx);
    perimeter_idx += 1;

    // Right edge: top → bottom
    for y in (y0 + 1)..y1 {
        set_cell(buf, x1, y, '│', perimeter_idx);
        perimeter_idx += 1;
    }

    // Bottom edge: right → left
    set_cell(buf, x1, y1, '┘', perimeter_idx);
    perimeter_idx += 1;
    for x in ((x0 + 1)..x1).rev() {
        set_cell(buf, x, y1, '─', perimeter_idx);
        perimeter_idx += 1;
    }
    set_cell(buf, x0, y1, '└', perimeter_idx);
    perimeter_idx += 1;

    // Left edge: bottom → top
    for y in ((y0 + 1)..y1).rev() {
        set_cell(buf, x0, y, '│', perimeter_idx);
        perimeter_idx += 1;
    }
}
