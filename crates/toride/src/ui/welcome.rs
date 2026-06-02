use std::time::Instant;

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use tachyonfx::{Interpolatable, color_from_hsl, color_to_hsl};

use crate::action::Action;
use crate::ui::gradient::GradientCache;
use crate::ui::responsive::{self, Viewport};
use crate::ui::theme::{self, Palette};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const EDITION: &str = "SINGLE-HOST";

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
    gradient_cache: GradientCache,
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
            gradient_cache: GradientCache::new(),
            anim_start: Instant::now(),
            color_cycle: build_color_cycle(theme::CHARM.accent),
        }
    }

    pub fn invalidate_cache(&mut self) {
        self.gradient_cache.invalidate();
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
        self.gradient_cache.render_or_copy(buf, area, p);

        // Adaptive center column
        let center = responsive::center_area(area);

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
        ] = ratatui::layout::Layout::vertical([
            ratatui::layout::Constraint::Fill(1),
            ratatui::layout::Constraint::Length(6),
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Fill(1),
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
        let key_style = p.key_style();
        let lbl_style = p.label_style();

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
        if let Some(cell) = buf.cell_mut(ratatui::layout::Position::new(x, y)) {
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
