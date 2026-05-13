use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::tui::model::Model;
use crate::tui::theme::SemanticToken;

pub fn render(area: Rect, frame: &mut Frame, model: &Model) {
    let theme = &model.theme;

    let spec = match model.current_screen() {
        crate::tui::model::Screen::Confirm(s) => s,
        _ => return,
    };

    frame.render_widget(Clear, area);

    let chunks = Layout::vertical([
        Constraint::Percentage(40),
        Constraint::Min(5),
        Constraint::Length(3),
        Constraint::Percentage(40),
    ]).split(area);

    let body = Paragraph::new(format!(
        "{}\n\n{}\n\n{}\n\n  [{}] {}    [{}] {}",
        spec.action_label,
        spec.description,
        if spec.is_destructive { "⚠ This action may be destructive" } else { "" },
        if true { "Enter" } else { " " },
        spec.confirm_label,
        "Esc",
        spec.cancel_label,
    ))
    .style(theme.style(SemanticToken::FgPrimary))
    .block(Block::default()
        .borders(Borders::ALL)
        .border_style(theme.style(SemanticToken::BorderFocus))
        .title(" Confirm "));

    frame.render_widget(body, chunks[1]);
}
