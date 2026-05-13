use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::tui::glyphs::Glyph;
use crate::tui::model::Model;
use crate::tui::theme::SemanticToken;

pub fn render(area: Rect, frame: &mut Frame, model: &Model) {
    let theme = &model.theme;
    let unicode = theme.unicode;

    let items: Vec<ListItem> = model.selection.modules.values().map(|m| {
        let glyph = if m.selected {
            Glyph::Checked
        } else {
            Glyph::Unchecked
        };
        let label = format!("{} {}", glyph.char(unicode), m.id.label());
        let style = if m.selected {
            theme.style(SemanticToken::FgPrimary)
        } else {
            theme.style(SemanticToken::FgMuted)
        };
        ListItem::new(label).style(style)
    }).collect();

    let scroll = model.list_scroll.min(items.len().saturating_sub(1));
    let list = List::new(items)
        .block(Block::default()
            .title("Modules")
            .borders(Borders::NONE));

    let mut state = ListState::default();
    state.select(Some(scroll));

    frame.render_stateful_widget(list, area, &mut state);
}
