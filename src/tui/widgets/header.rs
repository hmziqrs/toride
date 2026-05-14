use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::tui::model::Model;
use crate::tui::theme::SemanticToken;

pub fn render(area: Rect, frame: &mut Frame, model: &Model) {
    let theme = &model.theme;

    let os_badge = if model.system.os_name.is_empty() {
        String::new()
    } else {
        format!(" {} ", model.system.os_name)
    };

    let title = Line::from(vec![
        Span::styled(" Toride ", theme.styled(SemanticToken::Accent, ratatui::style::Modifier::BOLD)),
        Span::raw(" "),
        Span::styled(os_badge, theme.style(SemanticToken::FgMuted)),
    ]);

    let header = Paragraph::new(title)
        .style(Style::default().bg(theme.color(SemanticToken::BgBase)));

    frame.render_widget(header, area);
}
