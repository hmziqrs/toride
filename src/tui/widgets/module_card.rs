use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::tui::model::Model;
use crate::tui::theme::SemanticToken;

pub fn render(area: Rect, frame: &mut Frame, model: &Model) {
    let theme = &model.theme;

    let expanded_module = model.selection.modules.values().find(|m| m.expanded);
    let content = if let Some(m) = expanded_module {
        format!("{}\n\nModule details for {}", m.id.label(), m.id.label())
    } else {
        "Select a module and press Enter to expand".into()
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(theme.style(SemanticToken::Border))
            .title("Details"))
        .wrap(Wrap { trim: true })
        .style(theme.style(SemanticToken::FgSecondary));

    frame.render_widget(paragraph, area);
}
