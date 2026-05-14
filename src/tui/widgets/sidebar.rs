use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::tui::model::{Category, Model};
use crate::tui::theme::SemanticToken;

pub fn render(area: Rect, frame: &mut Frame, model: &Model) {
    let theme = &model.theme;

    let items: Vec<ListItem> = Category::all().iter().map(|cat| {
        let count = model.selection.modules.values()
            .filter(|m| m.id.category() == *cat && m.selected)
            .count();
        let total = model.selection.modules.values()
            .filter(|m| m.id.category() == *cat)
            .count();
        let label = format!("{} ({}/{})", cat.label(), count, total);
        let style = if count > 0 {
            theme.style(SemanticToken::FgPrimary)
        } else {
            theme.style(SemanticToken::FgMuted)
        };
        ListItem::new(label).style(style)
    }).collect();

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::RIGHT)
            .border_style(theme.style(SemanticToken::Border)));

    frame.render_widget(list, area);
}
