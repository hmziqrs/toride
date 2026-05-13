use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::tui::model::{Model, PlanActionStatus};
use crate::tui::theme::SemanticToken;

pub fn render(area: Rect, frame: &mut Frame, model: &Model) {
    let theme = &model.theme;

    let items: Vec<ListItem> = if let Some(ref plan) = model.plan {
        plan.actions.iter().map(|action| {
            let (prefix, style) = match action.status {
                PlanActionStatus::Pending => ("[ ] ", theme.style(SemanticToken::FgMuted)),
                PlanActionStatus::Running => ("[~] ", theme.style(SemanticToken::Accent)),
                PlanActionStatus::Done => ("[✓] ", theme.style(SemanticToken::Success)),
                PlanActionStatus::Failed => ("[✗] ", theme.style(SemanticToken::Danger)),
                PlanActionStatus::Skipped => ("[-] ", theme.style(SemanticToken::FgMuted)),
            };
            ListItem::new(format!("{}{}", prefix, action.label)).style(style)
        }).collect()
    } else {
        vec![ListItem::new("No plan generated yet").style(theme.style(SemanticToken::FgMuted))]
    };

    let list = List::new(items)
        .block(Block::default()
            .title("Apply Progress")
            .borders(Borders::ALL)
            .border_style(theme.style(SemanticToken::Border)));

    frame.render_widget(list, area);
}
