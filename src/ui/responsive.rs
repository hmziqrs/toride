use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use super::theme::Palette;

// ── Breakpoints ──────────────────────────────────────────────────────────────

pub const MIN_WIDTH: u16 = 30;
pub const MIN_HEIGHT: u16 = 10;

const FULL_W: u16 = 72;
const FULL_H: u16 = 24;
const COMPACT_W: u16 = 50;
const COMPACT_H: u16 = 16;

/// Terminal size category for adaptive layouts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Viewport {
    /// >= 72 x 24 — full chrome, generous spacing
    Full,
    /// >= 50 x 16 — reduced spacing, truncated content
    Compact,
    /// >= 30 x 10 — minimal: abbreviated text, no labels
    Minimal,
    /// < 30 x 10 — too small to render anything useful
    TooSmall,
}

impl Viewport {
    pub fn from_area(area: Rect) -> Self {
        let (w, h) = (area.width, area.height);
        if w < MIN_WIDTH || h < MIN_HEIGHT {
            Self::TooSmall
        } else if w >= FULL_W && h >= FULL_H {
            Self::Full
        } else if w >= COMPACT_W && h >= COMPACT_H {
            Self::Compact
        } else {
            Self::Minimal
        }
    }
}

// ── Layout helpers ───────────────────────────────────────────────────────────

/// Center column constraint — caps at 72 columns but shrinks on narrow terminals.
pub fn center_column() -> Constraint {
    Constraint::Max(FULL_W)
}

// ── Fallback rendering ───────────────────────────────────────────────────────

/// Render a centered "terminal too small" message.
/// Returns `true` when the terminal is too small — callers should early-return.
pub fn render_too_small(frame: &mut Frame, p: Palette) -> bool {
    let vp = Viewport::from_area(frame.area());
    if vp != Viewport::TooSmall {
        return false;
    }

    let area = frame.area();
    let msg = format!(
        "Terminal too small — need at least {}x{}",
        MIN_WIDTH, MIN_HEIGHT
    );
    let line = Line::from(Span::styled(msg, Style::new().fg(p.text_dim)));
    frame.render_widget(Paragraph::new(line).centered(), area);
    true
}

// ── Content truncation ───────────────────────────────────────────────────────

/// Truncate a string to `max_width` Unicode-width columns, appending ".." if it
/// doesn't fit. Handles multi-byte characters correctly.
pub fn truncate_str(s: &str, max_width: usize) -> String {
    if max_width < 2 {
        return String::new();
    }

    let mut width = 0usize;
    for (i, ch) in s.char_indices() {
        let cw = unicode_width(ch);
        if width + cw > max_width - 2 {
            let end = i;
            return format!("{}..", &s[..end]);
        }
        width += cw;
    }
    // Fits as-is.
    s.to_owned()
}

/// Truncate each line of a logo (array of `&str`) to `max_width`, appending ".."
/// when a row overflows. Returns owned `Line`s styled with the given style.
pub fn truncate_logo<'a>(
    rows: &[&'a str],
    max_width: u16,
    style: Style,
) -> Vec<Line<'a>> {
    let mw = max_width as usize;
    rows.iter()
        .map(|row| {
            let truncated = truncate_str(row, mw);
            // Re-borrow from the original if unchanged so we avoid allocating.
            let text = if truncated.len() == row.len() && truncated == *row {
                Cow::Borrowed(*row)
            } else {
                Cow::Owned(truncated)
            };
            Line::from(Span::styled(text, style))
        })
        .collect()
}

// ── Internal ─────────────────────────────────────────────────────────────────

fn unicode_width(ch: char) -> usize {
    match ch {
        '\0'..='\x7f' => 1,
        // CJK / wide — most terminals render at double width
        '\u{1100}'..='\u{115f}' => 2,
        '\u{2329}'..='\u{232a}' => 2,
        '\u{2e80}'..='\u{303e}' => 2,
        '\u{3041}'..='\u{3247}' => 2,
        '\u{3251}'..='\u{4dbf}' => 2,
        '\u{4e00}'..='\u{a4c6}' => 2,
        '\u{a960}'..='\u{a97c}' => 2,
        '\u{ac00}'..='\u{d7a3}' => 2,
        '\u{f900}'..='\u{faff}' => 2,
        '\u{fe10}'..='\u{fe19}' => 2,
        '\u{fe30}'..='\u{fe6b}' => 2,
        '\u{ff01}'..='\u{ff60}' => 2,
        '\u{ffe0}'..='\u{ffe6}' => 2,
        '\u{1f000}'..='\u{1f9ff}' => 2,
        _ => 1,
    }
}

use std::borrow::Cow;
