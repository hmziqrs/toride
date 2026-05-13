use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

use crate::tui::model::{Model, PaletteCmd};
use crate::tui::theme::SemanticToken;

pub fn render(area: Rect, frame: &mut Frame, model: &Model) {
    let theme = &model.theme;

    frame.render_widget(Clear, area);

    let query = model.palette_query.as_deref().unwrap_or("");
    let items: Vec<ListItem> = PaletteCmd::all().iter()
        .filter(|cmd| query.is_empty() || cmd.label().contains(query) || cmd.description().to_lowercase().contains(&query.to_lowercase()))
        .map(|cmd| {
            let style = theme.style(SemanticToken::FgPrimary);
            ListItem::new(format!("{} - {}", cmd.label(), cmd.description())).style(style)
        })
        .collect();

    let input = Paragraph::new(format!(":{}", query))
        .style(theme.style(SemanticToken::Accent))
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(theme.style(SemanticToken::BorderFocus))
            .title("Command Palette"));

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
