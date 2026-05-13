use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::tui::model::Model;
use crate::tui::theme::SemanticToken;

pub fn render(area: Rect, frame: &mut Frame, model: &Model) {
    let theme = &model.theme;

    let items: Vec<ListItem> = model.log.iter().rev().take(area.height as usize).map(|line| {
        let style = match line.level {
            crate::tui::model::LogLevel::Info => theme.style(SemanticToken::FgSecondary),
            crate::tui::model::LogLevel::Warn => theme.style(SemanticToken::Warning),
            crate::tui::model::LogLevel::Error => theme.style(SemanticToken::Danger),
        };
        ListItem::new(line.message.clone()).style(style)
    }).collect();

    let list = List::new(items)
        .block(Block::default()
            .title("Logs")
            .borders(Borders::ALL)
            .border_style(theme.style(SemanticToken::Border)));

    frame.render_widget(list, area);
}
