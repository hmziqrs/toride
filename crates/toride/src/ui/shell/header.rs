//! Top header bar: logo, inline cpu/ram/disk gauges, and a right-aligned clock.

use std::time::Instant;

use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::ui::helpers::percent_color;
use crate::ui::theme::Palette;

/// Data needed to render the header bar.
pub struct HeaderData<'a> {
    /// CPU usage percentage (0–100), if known.
    pub cpu: Option<f64>,
    /// RAM usage percentage (0–100), if known.
    pub ram: Option<f64>,
    /// Disk usage percentage (0–100), if known.
    pub disk: Option<f64>,
    /// Right-aligned clock label (e.g. `09:17 PM`).
    pub clock: &'a str,
    /// Timestamp used to drive the logo shimmer animation.
    pub shimmer_start: Instant,
}

/// Render the header bar into `area`.
pub fn render_header(frame: &mut Frame, area: Rect, p: Palette, data: &HeaderData) {
    let block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_style(Style::new().fg(p.border))
        .style(Style::new().bg(p.bg_alt));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sep = Span::styled("   ·   ", Style::new().fg(p.text_muted));

    let mut left = vec![
        Span::styled(" 砦 ", Style::new().fg(p.accent2).bold()),
        Span::styled("toride", Style::new().fg(p.text).bold()),
        sep.clone(),
    ];
    left.extend(gauge_spans("cpu", data.cpu, p));
    left.push(sep.clone());
    left.extend(gauge_spans("ram", data.ram, p));
    left.push(sep);
    left.extend(gauge_spans("disk", data.disk, p));

    frame.render_widget(Paragraph::new(Line::from(left)), inner);

    // Shimmer sweep across the logo (" 砦 toride" = 9 cells in header row 1).
    let elapsed = data.shimmer_start.elapsed().as_secs_f32();
    apply_logo_shimmer(frame.buffer_mut(), inner, elapsed);

    let clock = Line::from(Span::styled(
        format!("{} ", data.clock),
        Style::new().fg(p.text_dim),
    ));
    frame.render_widget(Paragraph::new(clock).right_aligned(), inner);
}

/// Pulsing brightness on the kanji logo cell.
fn apply_logo_shimmer(buf: &mut ratatui::buffer::Buffer, inner: Rect, elapsed: f32) {
    use tachyonfx::ColorSpace;

    // " 砦 " → kanji is at inner.x + 1
    let kanji_x = inner.x + 1;
    if kanji_x >= inner.right() || inner.y >= buf.area.height {
        return;
    }

    let pulse = (elapsed * 1.8f32).sin() * 0.5 + 0.5; // 0..1 oscillation
    if pulse < 0.01 {
        return;
    }

    let cell = &mut buf[Position::new(kanji_x, inner.y)];
    let fg = cell.fg;
    let lightened = ColorSpace::Hsl.lighten(&fg, pulse * 0.35);
    cell.set_fg(lightened);
}

/// Build the spans for one inline gauge (`cpu ▮ 35%`).
fn gauge_spans(label: &str, pct: Option<f64>, p: Palette) -> Vec<Span<'static>> {
    let (glyph_color, text): (Color, String) = match pct {
        Some(v) => (percent_color(v, p), format!("{v:.0}%")),
        None => (p.text_muted, "—".to_string()),
    };
    vec![
        Span::styled(format!("{label} "), Style::new().fg(p.text_dim)),
        Span::styled("▮ ", Style::new().fg(glyph_color)),
        Span::styled(text, Style::new().fg(p.text).bold()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::CHARM;
    use ratatui::{Terminal, backend::TestBackend};

    fn render(data: &HeaderData) -> String {
        let mut terminal = Terminal::new(TestBackend::new(60, 3)).unwrap();
        terminal
            .draw(|f| render_header(f, f.area(), CHARM, data))
            .unwrap();
        terminal.backend().to_string()
    }

    fn header_data(cpu: Option<f64>, ram: Option<f64>, disk: Option<f64>, clock: &str) -> HeaderData<'_> {
        HeaderData {
            cpu,
            ram,
            disk,
            clock,
            shimmer_start: Instant::now(),
        }
    }

    #[test]
    fn renders_logo_and_clock() {
        let out = render(&header_data(Some(35.0), Some(23.0), Some(23.0), "09:17 PM"));
        assert!(out.contains("toride"), "logo: {out}");
        assert!(out.contains("09:17 PM"), "clock: {out}");
        assert!(out.contains("35%"), "cpu gauge: {out}");
    }

    #[test]
    fn renders_dash_when_unknown() {
        let out = render(&header_data(None, None, None, "--:--"));
        assert!(out.contains('—'), "expected em-dash placeholder: {out}");
    }
}
