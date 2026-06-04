//! Reusable modal overlay widget.
//!
//! Renders a centered, bordered popup on top of existing screen content with a
//! dimmed scrim backdrop. Any content can be rendered inside via a closure.
//!
//! # Example
//!
//! ```ignore
//! use crate::ui::widgets::Modal;
//!
//! Modal::new("Confirm")
//!     .dimensions(40, 8)
//!     .render(frame, palette, |frame, content_area| {
//!         // render anything inside the modal
//!     });
//! ```

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Clear},
};

use crate::ui::theme::Palette;

/// Default modal dimensions.
const DEFAULT_WIDTH: u16 = 52;
const DEFAULT_HEIGHT: u16 = 16;

/// Scrim blend factors (how much to dim bg / fg toward the scrim target).
const SCRIM_BG_FACTOR: f32 = 0.55;
const SCRIM_FG_FACTOR: f32 = 0.45;

// ── Modal widget ───────────────────────────────────────────────────────────────

/// A reusable modal overlay widget.
///
/// Handles the full overlay pipeline: dimmed scrim, centered bordered box with
/// solid background, and arbitrary content rendering inside. Construct with
/// [`Modal::new`] and optional builder methods, then call [`Modal::render`].
pub struct Modal {
    title: &'static str,
    width: u16,
    height: u16,
}

impl Modal {
    /// Create a new modal with the given title displayed in the border.
    #[must_use]
    pub fn new(title: &'static str) -> Self {
        Self {
            title,
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
        }
    }

    /// Set the modal dimensions (clamped to terminal size at render time).
    #[must_use]
    pub fn dimensions(mut self, width: u16, height: u16) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Render the modal overlay.
    ///
    /// This performs the full pipeline:
    /// 1. Dims all existing buffer cells **outside** the modal rect (scrim)
    /// 2. Clears the modal area so the block background is opaque
    /// 3. Renders a bordered box with the title
    /// 4. Calls `content_fn` with the inner content area
    pub fn render(self, frame: &mut Frame, palette: Palette, content_fn: impl FnOnce(&mut Frame, Rect)) {
        let area = frame.area();
        let modal_rect = Self::centered_rect(self.width, self.height, area);

        // 1. Dimmed scrim — blend existing cells toward a darkened bg, skip modal rect
        let scrim_target = dim_color(palette.bg);
        let buf = frame.buffer_mut();
        let area_w = area.width as usize;
        for (i, cell) in buf.content.iter_mut().enumerate() {
            let x = area.x + (i % area_w) as u16;
            let y = area.y + (i / area_w) as u16;
            if x >= modal_rect.left()
                && x < modal_rect.right()
                && y >= modal_rect.top()
                && y < modal_rect.bottom()
            {
                continue;
            }
            cell.set_bg(blend_toward(cell.bg, scrim_target, SCRIM_BG_FACTOR));
            cell.set_fg(blend_toward(cell.fg, scrim_target, SCRIM_FG_FACTOR));
        }

        // 2. Clear modal area so block bg fills every cell
        frame.render_widget(Clear, modal_rect);

        // 3. Bordered box with title
        let block = Block::bordered()
            .title(format!(" {} ", self.title))
            .title_alignment(ratatui::layout::Alignment::Center)
            .border_style(Style::new().fg(palette.border_hi))
            .style(Style::new().bg(palette.panel));
        let content_area = block.inner(modal_rect);
        frame.render_widget(block, modal_rect);

        // 4. Caller's content
        content_fn(frame, content_area);
    }

    /// Compute a centered rect using `Layout` constraints (ratatui best practice).
    fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
        let w = width.min(area.width);
        let h = height.min(area.height);

        let [_, row, _] = Layout::vertical([
            Constraint::Length((area.height.saturating_sub(h)) / 2),
            Constraint::Length(h),
            Constraint::Length((area.height.saturating_sub(h)) / 2),
        ])
        .areas(area);

        let [_, center, _] = Layout::horizontal([
            Constraint::Length((area.width.saturating_sub(w)) / 2),
            Constraint::Length(w),
            Constraint::Length((area.width.saturating_sub(w)) / 2),
        ])
        .areas(row);

        center
    }
}

// ── Color blending helpers ─────────────────────────────────────────────────────

/// Darken an RGB color to ~1/3 brightness (scrim blend target).
fn dim_color(color: ratatui::style::Color) -> ratatui::style::Color {
    match color {
        ratatui::style::Color::Rgb(r, g, b) => ratatui::style::Color::Rgb(r / 3, g / 3, b / 3),
        other => other,
    }
}

/// Linearly interpolate `color` toward `target` by `t` (0.0 = unchanged, 1.0 = target).
fn blend_toward(
    color: ratatui::style::Color,
    target: ratatui::style::Color,
    t: f32,
) -> ratatui::style::Color {
    let ratatui::style::Color::Rgb(cr, cg, cb) = color else {
        return color;
    };
    let ratatui::style::Color::Rgb(tr, tg, tb) = target else {
        return color;
    };
    #[expect(clippy::cast_lossless, reason = "u8→f32 for blending math")]
    let r = (cr as f32 + (tr as f32 - cr as f32) * t).round() as u8;
    #[expect(clippy::cast_lossless, reason = "u8→f32 for blending math")]
    let g = (cg as f32 + (tg as f32 - cg as f32) * t).round() as u8;
    #[expect(clippy::cast_lossless, reason = "u8→f32 for blending math")]
    let b = (cb as f32 + (tb as f32 - cb as f32) * t).round() as u8;
    ratatui::style::Color::Rgb(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_rect_fits_within_area() {
        let area = Rect::new(0, 0, 80, 24);
        let rect = Modal::centered_rect(52, 16, area);
        assert!(rect.width <= area.width);
        assert!(rect.height <= area.height);
        assert!(rect.x >= area.x);
        assert!(rect.y >= area.y);
    }

    #[test]
    fn centered_rect_clamps_to_area() {
        let area = Rect::new(0, 0, 30, 10);
        let rect = Modal::centered_rect(52, 16, area);
        assert_eq!(rect.width, 30);
        assert_eq!(rect.height, 10);
    }

    #[test]
    fn centered_rect_is_actually_centered() {
        let area = Rect::new(0, 0, 80, 24);
        let rect = Modal::centered_rect(40, 10, area);
        // 80-40=40, /2=20 → x should be 14 (leftover split)
        assert_eq!((rect.x - area.x) + rect.width + (area.width - rect.x - rect.width), area.width - area.x);
    }

    #[test]
    fn dim_color_darkens_rgb() {
        let dimmed = dim_color(ratatui::style::Color::Rgb(30, 20, 40));
        assert_eq!(dimmed, ratatui::style::Color::Rgb(10, 6, 13));
    }

    #[test]
    fn dim_color_passes_through_non_rgb() {
        let color = ratatui::style::Color::Red;
        assert_eq!(dim_color(color), color);
    }

    #[test]
    fn blend_toward_interpolates() {
        let from = ratatui::style::Color::Rgb(100, 50, 0);
        let target = ratatui::style::Color::Rgb(10, 6, 13);
        let result = blend_toward(from, target, 0.5);
        assert_eq!(result, ratatui::style::Color::Rgb(55, 28, 7));
    }

    #[test]
    fn blend_toward_zero_is_unchanged() {
        let color = ratatui::style::Color::Rgb(100, 200, 50);
        let target = ratatui::style::Color::Rgb(10, 6, 13);
        assert_eq!(blend_toward(color, target, 0.0), color);
    }

    #[test]
    fn blend_toward_one_is_target() {
        let color = ratatui::style::Color::Rgb(100, 200, 50);
        let target = ratatui::style::Color::Rgb(10, 6, 13);
        assert_eq!(blend_toward(color, target, 1.0), target);
    }

    #[test]
    fn blend_toward_passes_through_non_rgb_color() {
        let color = ratatui::style::Color::Red;
        let target = ratatui::style::Color::Rgb(10, 6, 13);
        assert_eq!(blend_toward(color, target, 0.5), color);
    }

    #[test]
    fn blend_toward_passes_through_non_rgb_target() {
        let color = ratatui::style::Color::Rgb(100, 200, 50);
        let target = ratatui::style::Color::Red;
        assert_eq!(blend_toward(color, target, 0.5), color);
    }

    #[test]
    fn new_sets_defaults() {
        let modal = Modal::new("Test");
        assert_eq!(modal.title, "Test");
        assert_eq!(modal.width, DEFAULT_WIDTH);
        assert_eq!(modal.height, DEFAULT_HEIGHT);
    }

    #[test]
    fn dimensions_overrides_size() {
        let modal = Modal::new("Test").dimensions(40, 8);
        assert_eq!(modal.width, 40);
        assert_eq!(modal.height, 8);
    }
}
