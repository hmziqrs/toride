use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

use crate::tui::model::{Model, ModuleId};
use crate::tui::theme::SemanticToken;

pub fn render(area: Rect, frame: &mut Frame, model: &Model) {
    let theme = &model.theme;

    frame.render_widget(Clear, area);

    let query = model.search_query.as_deref().unwrap_or("");

    let items: Vec<ListItem> = if query.is_empty() {
        ModuleId::all().iter().map(|id| {
            ListItem::new(format!("  {}", id.label())).style(theme.style(SemanticToken::FgMuted))
        }).collect()
    } else {
        let lq = query.to_lowercase();
        ModuleId::all().iter()
            .filter(|id| id.label().to_lowercase().contains(&lq))
            .map(|id| {
                let style = if model.selection.modules.get(id).map(|m| m.selected).unwrap_or(false) {
                    theme.styled(SemanticToken::Accent, ratatui::style::Modifier::BOLD)
                } else {
                    theme.style(SemanticToken::FgPrimary)
                };
                ListItem::new(format!("  {}", id.label())).style(style)
            }).collect()
    };

    let input = Paragraph::new(format!("/{}", query))
        .style(theme.style(SemanticToken::Accent))
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(theme.style(SemanticToken::BorderFocus))
            .title("Search modules"));

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(theme.style(SemanticToken::Border)));

    let chunks = ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Length(3),
        ratatui::layout::Constraint::Min(1),
    ]).split(area);

    frame.render_widget(input, chunks[0]);
    frame.render_widget(list, chunks[1]);
}
