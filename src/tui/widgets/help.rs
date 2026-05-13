use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};

use crate::tui::keymap;
use crate::tui::model::Model;
use crate::tui::theme::SemanticToken;

pub fn render(area: Rect, frame: &mut Frame, model: &Model) {
    let theme = &model.theme;

    frame.render_widget(Clear, area);

    let mut items: Vec<ListItem> = Vec::new();

    for b in keymap::global_bindings() {
        items.push(ListItem::new(format!("  {:<15} {}", b.key, b.description))
            .style(theme.style(SemanticToken::FgPrimary)));
    }

    items.push(ListItem::new("").style(theme.style(SemanticToken::FgMuted)));
    items.push(ListItem::new("  Module Selection").style(
        theme.styled(SemanticToken::Accent, ratatui::style::Modifier::BOLD)));

    for b in keymap::module_selection_bindings() {
        items.push(ListItem::new(format!("  {:<15} {}", b.key, b.description))
            .style(theme.style(SemanticToken::FgPrimary)));
    }

    let list = List::new(items)
        .block(Block::default()
            .title(" Help — press ? or Esc to close ")
            .borders(Borders::ALL)
            .border_style(theme.style(SemanticToken::BorderFocus)));

    frame.render_widget(list, area);
}
