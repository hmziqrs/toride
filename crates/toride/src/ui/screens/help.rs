use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::action::Action;
use crate::ui::responsive::{self, Viewport};
use crate::ui::screens::AppScreen;
use crate::ui::theme::Palette;
use crate::ui::widgets::gradient::GradientCache;

pub struct HelpScreen {
    gradient_cache: GradientCache,
}

impl Default for HelpScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl AppScreen for HelpScreen {
    fn handle_key(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Char('b') | KeyCode::Esc => Some(Action::Back),
            KeyCode::Char('q') => Some(Action::Quit),
            _ => None,
        }
    }

    fn view(&mut self, frame: &mut Frame, palette: Palette) {
        let area = frame.area();
        let viewport = Viewport::from_area(area);

        if responsive::render_too_small(frame, palette) {
            return;
        }

        let buf = frame.buffer_mut();
        self.gradient_cache.render_or_copy(buf, area, palette);

        Self::render_content(frame, palette, viewport);
    }

    fn view_foreground(&mut self, frame: &mut Frame, palette: Palette) {
        let area = frame.area();
        let viewport = Viewport::from_area(area);

        if responsive::render_too_small(frame, palette) {
            return;
        }

        Self::render_content(frame, palette, viewport);
    }

    fn invalidate_cache(&mut self) {
        self.gradient_cache.invalidate();
    }

    fn needs_animation(&self) -> bool {
        false
    }
}

impl HelpScreen {
    pub fn new() -> Self {
        Self {
            gradient_cache: GradientCache::new(),
        }
    }

    fn render_content(frame: &mut Frame, p: Palette, viewport: Viewport) {
        let area = frame.area();
        let center = responsive::center_area(area);

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
        let key_style = p.key_style();
        let lbl_style = p.label_style();

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

fn keybinding_line<'a>(key: &str, desc: &str, key_style: Style, lbl_style: Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!(" {key} "), key_style),
        Span::raw("  "),
        Span::styled(desc.to_string(), lbl_style),
    ])
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;

    use super::HelpScreen;
    use crate::action::Action;
    use crate::ui::screens::AppScreen;

    #[test]
    fn new_creates_screen() {
        let _screen = HelpScreen::new();
    }

    #[test]
    fn handle_key_back_on_b() {
        let mut screen = HelpScreen::new();
        assert_eq!(screen.handle_key(KeyCode::Char('b')), Some(Action::Back));
    }

    #[test]
    fn handle_key_back_on_esc() {
        let mut screen = HelpScreen::new();
        assert_eq!(screen.handle_key(KeyCode::Esc), Some(Action::Back));
    }

    #[test]
    fn handle_key_quit_on_q() {
        let mut screen = HelpScreen::new();
        assert_eq!(screen.handle_key(KeyCode::Char('q')), Some(Action::Quit));
    }

    #[test]
    fn handle_key_none_for_other_keys() {
        let mut screen = HelpScreen::new();
        assert_eq!(screen.handle_key(KeyCode::Char('a')), None);
        assert_eq!(screen.handle_key(KeyCode::Enter), None);
        assert_eq!(screen.handle_key(KeyCode::Up), None);
    }

    #[test]
    fn needs_animation_returns_false() {
        let screen = HelpScreen::new();
        assert!(!screen.needs_animation());
    }
}
