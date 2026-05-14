use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};

use crate::tui::model::{Model, Screen};
use crate::tui::theme::SemanticToken;

pub fn view(frame: &mut Frame, model: &Model) {
    let area = frame.area();

    if area.width < 80 || area.height < 24 {
        render_too_small(frame, area, model);
        return;
    }

    let screen = *model.current_screen();

    if screen.is_overlay() {
        render_base(frame, area, model);
        render_overlay(frame, area, model, screen);
    } else {
        render_base(frame, area, model);
    }
}

fn render_base(frame: &mut Frame, area: Rect, model: &Model) {
    let screen = *model.current_screen();

    let vertical = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ]);
    let [header_area, body_area, status_area] = vertical.areas(area);

    crate::tui::widgets::header::render(header_area, frame, model);
    crate::tui::widgets::status_bar::render(status_area, frame, model);
    crate::tui::widgets::toast::render(body_area, frame, model);

    if model.log_panel_visible {
        let [content_area, log_area] = Layout::vertical([
            Constraint::Percentage(70),
            Constraint::Percentage(30),
        ]).areas(body_area);
        render_body(frame, content_area, model, screen);
        crate::tui::widgets::log_view::render(log_area, frame, model);
    } else {
        render_body(frame, body_area, model, screen);
    }
}

fn render_body(frame: &mut Frame, area: Rect, model: &Model, screen: Screen) {
    match screen {
        Screen::Welcome => {
            crate::tui::widgets::splash::render(area, frame, model);
        }
        Screen::ProfileSelect => {
            render_profile_select(frame, area, model);
        }
        Screen::ModuleSelect => {
            if area.width >= 100 {
                let [sidebar_area, content_area] = Layout::horizontal([
                    Constraint::Length(24),
                    Constraint::Min(1),
                ]).areas(area);
                crate::tui::widgets::sidebar::render(sidebar_area, frame, model);
                let [list_area, card_area] = Layout::horizontal([
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ]).areas(content_area);
                crate::tui::widgets::module_list::render(list_area, frame, model);
                crate::tui::widgets::module_card::render(card_area, frame, model);
            } else {
                crate::tui::widgets::module_list::render(area, frame, model);
            }
        }
        Screen::Configure => {
            render_configure(frame, area, model);
        }
        Screen::Preflight => {
            crate::tui::widgets::progress_panel::render(area, frame, model);
        }
        Screen::Apply => {
            crate::tui::widgets::progress_panel::render(area, frame, model);
        }
        Screen::Summary => {
            render_summary(frame, area, model);
        }
        _ => {}
    }
}

fn render_overlay(frame: &mut Frame, area: Rect, model: &Model, screen: Screen) {
    let overlay_area = centered_rect(area, 60, 70);
    match screen {
        Screen::Help => crate::tui::widgets::help::render(overlay_area, frame, model),
        Screen::Palette => crate::tui::widgets::palette::render(overlay_area, frame, model),
        Screen::Search => {
            let search_area = Rect { x: area.x, y: area.y, width: area.width, height: 3 };
            crate::tui::widgets::search::render(search_area, frame, model);
        }
        Screen::Confirm(_) => crate::tui::widgets::confirm::render(overlay_area, frame, model),
        _ => {}
    }
}

fn render_profile_select(frame: &mut Frame, area: Rect, model: &Model) {
    let theme = &model.theme;
    use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
    use crate::tui::theme::SemanticToken;

    let profiles = [
        ("Basic", "Secure production-ready VPS setup"),
        ("Custom", "Manually choose every module"),
    ];

    let items: Vec<ListItem> = profiles.iter().enumerate().map(|(i, (name, desc))| {
        let selected = match model.profile {
            Some(crate::tui::model::Profile::Basic) => i == 0,
            Some(crate::tui::model::Profile::Custom) => i == 1,
            None => i == 0,
        };
        let style = if selected {
            theme.styled(SemanticToken::Accent, ratatui::style::Modifier::BOLD)
        } else {
            theme.style(SemanticToken::FgSecondary)
        };
        ListItem::new(format!("  {}    {}", name, desc)).style(style)
    }).collect();

    let list = List::new(items)
        .block(Block::default()
            .title(" Choose setup profile ")
            .borders(Borders::ALL)
            .border_style(theme.style(SemanticToken::Border)));

    let mut state = ListState::default();
    state.select(Some(0));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_configure(frame: &mut Frame, area: Rect, model: &Model) {
    use ratatui::widgets::{Block, Borders, Paragraph};
    let theme = &model.theme;

    let content = format!(
        "Configuration\n\nUsername: {}\nSSH Key: {}\nSwap Size: {}\nSSH Port: 22\n\nPress Tab to switch fields, Enter to continue",
        model.forms.get(crate::tui::model::FormField::Username),
        if model.forms.get(crate::tui::model::FormField::SshPublicKey).is_empty() { "(not set)" } else { "configured" },
        if model.forms.get(crate::tui::model::FormField::SwapSize).is_empty() { "2G (default)" } else { model.forms.get(crate::tui::model::FormField::SwapSize) },
    );

    let paragraph = Paragraph::new(content)
        .block(Block::default()
            .title(" Configuration ")
            .borders(Borders::ALL)
            .border_style(theme.style(SemanticToken::Border)))
        .style(theme.style(SemanticToken::FgPrimary));

    frame.render_widget(paragraph, area);
}

fn render_summary(frame: &mut Frame, area: Rect, model: &Model) {
    use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
    let theme = &model.theme;

    let selected: Vec<String> = model.selection.selected_ids()
        .iter().map(|id| id.label().to_string()).collect();

    let reboot_line = if model.reboot_required {
        "\n\nREBOOT REQUIRED /var/run/reboot-required is present."
    } else {
        ""
    };

    let outcome_line = match &model.run {
        crate::tui::model::RunState::Done(crate::tui::model::Outcome::Success) => "All steps completed successfully.".into(),
        crate::tui::model::RunState::Done(crate::tui::model::Outcome::PartialSuccess { failed }) => {
            format!("Completed with {} failure(s).", failed.len())
        }
        crate::tui::model::RunState::Done(crate::tui::model::Outcome::Failed { error }) => {
            format!("Setup failed: {}", error)
        }
        crate::tui::model::RunState::Done(crate::tui::model::Outcome::Cancelled) => "Setup was cancelled.".into(),
        _ => String::new(),
    };

    let content = format!(
        "Setup Complete\n\n{}\n\nSelected modules:\n{}{}\n\nPress q to quit",
        outcome_line,
        if selected.is_empty() { "  (none)".into() } else { selected.iter().map(|s| format!("  + {}", s)).collect::<Vec<_>>().join("\n") },
        reboot_line,
    );

    let paragraph = Paragraph::new(content)
        .block(Block::default()
            .title(" Summary ")
            .borders(Borders::ALL)
            .border_style(theme.style(SemanticToken::Border)))
        .style(theme.style(SemanticToken::FgPrimary))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

fn render_too_small(frame: &mut Frame, area: Rect, _model: &Model) {
    use ratatui::widgets::Paragraph;
    let paragraph = Paragraph::new("Terminal too small. Please resize to at least 80x24.");
    frame.render_widget(paragraph, area);
}

fn centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ]).split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ]).split(popup_layout[1])[1]
}
