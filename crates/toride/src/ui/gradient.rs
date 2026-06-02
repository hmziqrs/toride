use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::Color,
};

use super::theme::Palette;

// ── Gradient cache ─────────────────────────────────────────────────────────────

/// Caches gradient background colours keyed by area, avoiding recomputation
/// when the terminal size hasn't changed.
pub struct GradientCache {
    cached_area: Option<Rect>,
    colors: Vec<Color>,
}

impl GradientCache {
    /// Create an empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cached_area: None,
            colors: Vec::new(),
        }
    }

    /// Invalidate the cache so the next render regenerates the gradient.
    pub fn invalidate(&mut self) {
        self.cached_area = None;
    }

    /// Render the gradient into the cache (if the area changed) and copy
    /// background colours to `buf`.
    pub fn render_or_copy(&mut self, buf: &mut Buffer, area: Rect, p: Palette) {
        let needs_regen = self.cached_area != Some(area);
        if needs_regen {
            render_gradient_bg(&mut self.colors, area, p);
            self.cached_area = Some(area);
        }
        copy_bg_to_buf(&self.colors, buf, area);
    }
}

impl Default for GradientCache {
    fn default() -> Self {
        Self::new()
    }
}

// ── Gradient computation ───────────────────────────────────────────────────────

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    reason = "color math: f64->u8 truncation is intentional for RGB blending"
)]
fn render_gradient_bg(colors: &mut Vec<Color>, area: Rect, p: Palette) {
    colors.clear();
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
            colors.push(Color::Rgb(r, g, b));
        }
    }
}

/// Copy cached background colours directly to the frame buffer, using direct
/// indexing to avoid the double bounds check of `cell()` / `cell_mut()`.
fn copy_bg_to_buf(colors: &[Color], buf: &mut Buffer, area: Rect) {
    let mut i = 0usize;
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if i < colors.len() {
                buf[Position::new(x, y)].set_bg(colors[i]);
            }
            i += 1;
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn rgb_components(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (0, 0, 0),
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a * (1.0 - t) + b * t
}
