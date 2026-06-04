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
    /// Disk I/O throughput label (e.g. `"50↓ 20↑ MB/s"`), if known.
    pub disk: Option<&'a str>,
    /// Network usage label (e.g. `"12 MB/s"`), if known.
    pub net: Option<&'a str>,
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
        Span::styled(" 砦 ", Style::new().fg(p.accent).bold()),
        Span::styled("toride", Style::new().fg(p.accent).bold()),
        sep.clone(),
    ];
    left.extend(pct_gauge_spans("cpu", data.cpu, p));
    left.push(sep.clone());
    left.extend(pct_gauge_spans("ram", data.ram, p));
    left.push(sep.clone());
    if let Some(disk_label) = data.disk {
        left.extend(gauge_spans("disk", disk_label, p.accent3, p));
    } else {
        left.extend(gauge_spans("disk", "—", p.text_muted, p));
    }
    if let Some(net_label) = data.net {
        left.push(sep);
        left.extend(gauge_spans("net", net_label, p.accent3, p));
    }

    frame.render_widget(Paragraph::new(Line::from(left)), inner);

    // Shimmer sweep across the logo (" 砦 toride" = 9 cells in header row 1).
    let elapsed = data.shimmer_start.elapsed().as_secs_f32();
    apply_logo_shimmer(frame.buffer_mut(), inner, elapsed);
    apply_loading_pulse(frame.buffer_mut(), inner, elapsed, p);

    let clock = Line::from(Span::styled(
        format!("{} ", data.clock),
        Style::new().fg(p.text_dim),
    ));
    frame.render_widget(Paragraph::new(clock).right_aligned(), inner);
}

/// Gaussian brightness sweep across the kanji + "toride" logo cells.
fn apply_logo_shimmer(buf: &mut ratatui::buffer::Buffer, inner: Rect, elapsed: f32) {
    use tachyonfx::ColorSpace;

    // " 砦 " (CJK width-2 → 4 cols) + "toride" (6 cols) = 10 columns
    const LOGO_W: u16 = 10;
    let logo_end = (inner.x + LOGO_W).min(inner.right());

    let sweep_period = 3.0f32;
    let sweep_pos = (elapsed % sweep_period) / sweep_period;
    let sigma = 0.06f32;

    for x in inner.x..logo_end {
        let cell = &mut buf[Position::new(x, inner.y)];
        if cell.symbol() == " " {
            continue;
        }

        let cell_norm = f32::from(x - inner.x) / f32::from(LOGO_W);
        let dist = cell_norm - sweep_pos;
        let brightness = (-dist * dist / (2.0 * sigma * sigma)).exp();

        if brightness > 0.01 {
            let fg = cell.fg;
            let lightened = ColorSpace::Hsl.lighten(&fg, brightness * 0.4);
            cell.set_fg(lightened);
        }
    }
}

/// Subtle brightness pulse on "—" placeholder cells for gauges still loading.
fn apply_loading_pulse(buf: &mut ratatui::buffer::Buffer, inner: Rect, elapsed: f32, _p: Palette) {
    // Suppress unused-variable warning — palette kept for future tint options.
    use tachyonfx::ColorSpace;

    // 1.5 s cycle — smooth sine wave between 0 and 0.25.
    let alpha = ((elapsed * std::f32::consts::TAU / 1.5).sin() + 1.0) * 0.5 * 0.25;

    for x in inner.x..inner.right() {
        let cell = &mut buf[Position::new(x, inner.y)];
        if cell.symbol() == "—" {
            let fg = cell.fg;
            let lightened = ColorSpace::Hsl.lighten(&fg, alpha);
            cell.set_fg(lightened);
        }
    }
}

/// Compute the hitbox [`Rect`] for each gauge span within the header's inner row.
///
/// Returns `[cpu_rect, ram_rect, disk_rect, net_rect]`. `data` must match the
/// data passed to [`render_header`] for the same frame so the widths are
/// consistent.
#[must_use]
pub fn gauge_hitboxes(area: Rect, data: &HeaderData) -> [Rect; 4] {
    let block = Block::default().borders(Borders::TOP | Borders::BOTTOM);
    let inner = block.inner(area);

    // Walk the same span layout as render_header to find x offsets.
    let mut x = inner.x;
    // " 砦 " (CJK width-2 = 4 cols) + "toride" (6 cols) + "   ·   " (7 cols)
    x += 4 + 6 + 7;

    let labels = ["cpu", "ram"];
    let pcts = [data.cpu, data.ram];
    let mut rects = [Rect::default(); 4];

    for (i, (&label, &pct)) in labels.iter().zip(pcts.iter()).enumerate() {
        let w = pct_gauge_width(label, pct);
        rects[i] = Rect::new(x, inner.y, w, 1);
        x += w;
        x += 7; // separator "   ·   "
    }

    // Disk I/O gauge (index 2)
    let disk_val = data.disk.unwrap_or("—");
    let w = gauge_width("disk", disk_val);
    rects[2] = Rect::new(x, inner.y, w, 1);
    x += w;
    x += 7; // separator

    // Net gauge (index 3)
    if let Some(net_label) = data.net {
        let w = gauge_width("net", net_label);
        rects[3] = Rect::new(x, inner.y, w, 1);
    }

    rects
}

/// Unicode display width of the spans produced by [`gauge_spans`].
fn gauge_width(label: &str, value: &str) -> u16 {
    // "{label} " + "▮ " + "{value}"
    u16::try_from(label.len() + 1 + 2 + value.len()).unwrap_or(10)
}

/// Unicode display width of the spans produced by [`pct_gauge_spans`].
fn pct_gauge_width(label: &str, pct: Option<f64>) -> u16 {
    let text = match pct {
        Some(v) => format!("{v:.0}%"),
        None => "—".to_string(),
    };
    gauge_width(label, &text)
}

/// Build the spans for one inline gauge (`label ▮ value`).
fn gauge_spans(label: &str, value: &str, glyph_color: Color, p: Palette) -> Vec<Span<'static>> {
    vec![
        Span::styled(format!("{label} "), Style::new().fg(p.text_dim)),
        Span::styled("▮ ", Style::new().fg(glyph_color)),
        Span::styled(value.to_string(), Style::new().fg(p.text).bold()),
    ]
}

/// Build the spans for a percentage-based inline gauge (`cpu ▮ 35%`).
fn pct_gauge_spans(label: &str, pct: Option<f64>, p: Palette) -> Vec<Span<'static>> {
    let (glyph_color, text): (Color, String) = match pct {
        Some(v) => (percent_color(v, p), format!("{v:.0}%")),
        None => (p.text_muted, "—".to_string()),
    };
    gauge_spans(label, &text, glyph_color, p)
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

    fn header_data(cpu: Option<f64>, ram: Option<f64>, disk: Option<&'static str>, clock: &'static str) -> HeaderData<'static> {
        HeaderData {
            cpu,
            ram,
            disk,
            net: None,
            clock,
            shimmer_start: Instant::now(),
        }
    }

    #[test]
    fn renders_logo_and_clock() {
        let out = render(&header_data(Some(35.0), Some(23.0), Some("1.2↓ 0.5↑ MB"), "09:17 PM"));
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
