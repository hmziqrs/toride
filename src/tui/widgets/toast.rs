use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::tui::model::{Model, ToastKind};
use crate::tui::theme::SemanticToken;

pub fn render(area: Rect, frame: &mut Frame, model: &Model) {
    if model.toasts.is_empty() {
        return;
    }

    let theme = &model.theme;
    let toasts: Vec<Line> = model.toasts.iter().rev().take(3).map(|t| {
        let (prefix, token) = match t.kind {
            ToastKind::Info => ("INFO ", SemanticToken::Info),
            ToastKind::Success => ("OK ", SemanticToken::Success),
            ToastKind::Warning => ("WARN ", SemanticToken::Warning),
            ToastKind::Error => ("ERR ", SemanticToken::Danger),
        };
        Line::styled(
            format!(" {} {}", prefix, t.message),
            theme.bg_style(SemanticToken::BgOverlay, token),
        )
    }).collect();

    let height = toasts.len().min(3) as u16;
    let toast_area = Rect {
        x: area.width.saturating_sub(50),
        y: area.height.saturating_sub(height + 2),
        width: 50.min(area.width),
        height,
    };

    let paragraph = Paragraph::new(toasts);
    frame.render_widget(paragraph, toast_area);
}
