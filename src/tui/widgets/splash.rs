use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;

use crate::tui::model::Model;
use crate::tui::theme::SemanticToken;

pub fn render(area: Rect, frame: &mut Frame, model: &Model) {
    let theme = &model.theme;

    let content = format!(
        "\n  Toride\n  VPS Setup Tool\n\n  OS: {}\n  User: {}\n  Root: {}\n\n  Press Enter to continue...\n",
        if model.system.os_name.is_empty() { "Detecting...".into() } else { model.system.os_name.clone() },
        model.system.current_user,
        if model.system.is_root { "yes" } else { "no" },
    );

    let paragraph = Paragraph::new(content)
        .style(theme.style(SemanticToken::FgPrimary));

    frame.render_widget(paragraph, area);
}
