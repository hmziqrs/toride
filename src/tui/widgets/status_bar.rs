use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::tui::model::Model;
use crate::tui::theme::SemanticToken;

pub fn render(area: Rect, frame: &mut Frame, model: &Model) {
    let theme = &model.theme;
    let screen = *model.current_screen();

    let bindings = crate::tui::keymap::status_bar_hints(screen, model.caps.width);

    let mut spans: Vec<Span> = vec![Span::from(" ")];

    for b in &bindings {
        spans.push(Span::styled(format!(" {} ", b.key), theme.bg_style(SemanticToken::Accent, SemanticToken::FgInverse)));
        spans.push(Span::styled(format!(" {} ", b.description), theme.style(SemanticToken::FgMuted)));
    }

    if model.dry_run {
        spans.push(Span::styled(" DRY-RUN ", theme.bg_style(SemanticToken::Warning, SemanticToken::FgInverse)));
    }

    let line = Line::from(spans);
    let bar = Paragraph::new(line)
        .style(theme.bg_style(SemanticToken::BgRaised, SemanticToken::FgMuted));

    frame.render_widget(bar, area);
}
