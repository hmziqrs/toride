//! Quit confirmation modal.
//!
//! A self-contained modal that asks the user to confirm before quitting.
//! Owns its own interactive buttons, focus state, rendering, and input handling.

use crossterm::event::{KeyCode, MouseEvent};
use ratatui::{
    layout::{Constraint, Layout},
    widgets::Paragraph,
};

use crate::action::Action;
use crate::ui::components::{interactive_button::InteractiveButton, ButtonRow};
use crate::ui::responsive::Viewport;
use crate::ui::theme::Palette;
use crate::ui::widgets::Modal;

/// Horizontal gap between the two buttons.
const BTN_GAP: u16 = 4;

/// Self-contained quit confirmation modal.
///
/// Owns interactive Yes/No buttons with focus cycling and mouse support.
/// Delegates rendering to the shared [`Modal`] widget.
pub struct QuitModal {
    buttons: ButtonRow<Action>,
}

impl QuitModal {
    #[must_use]
    pub fn new() -> Self {
        let buttons = vec![
            InteractiveButton::new("yes", "y", Action::Quit),
            InteractiveButton::new("no", "n", Action::DismissQuit),
        ];

        Self {
            buttons: ButtonRow::new(buttons, vec![BTN_GAP, 0]),
        }
    }

    /// Handle a key press while the quit modal is open.
    pub fn handle_key(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            // Direct shortcuts
            KeyCode::Char('y') => Some(Action::Quit),
            KeyCode::Char('n') | KeyCode::Esc => Some(Action::DismissQuit),
            // Enter activates the focused button
            KeyCode::Enter => {
                Some(self.buttons.activate_focused().unwrap_or(Action::Quit))
            }
            // Focus cycling
            KeyCode::Tab | KeyCode::Right => {
                self.buttons.cycle_focus_next();
                None
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.buttons.cycle_focus_prev();
                None
            }
            _ => None,
        }
    }

    /// Handle a mouse event while the quit modal is open.
    pub fn handle_mouse(&mut self, mouse: &MouseEvent) -> Option<Action> {
        self.buttons.handle_mouse(mouse)
    }

    /// Render the quit modal overlay.
    pub fn render(&mut self, frame: &mut ratatui::Frame, p: Palette) {
        let viewport = Viewport::from_area(frame.area());

        Modal::new("Quit?")
            .dimensions(36, 7)
            .render(frame, p, |frame, content_area| {
                let [_, msg_area, _, keys_area, _] = Layout::vertical([
                    Constraint::Fill(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Fill(1),
                ])
                .areas(content_area);

                frame.render_widget(
                    Paragraph::new("Are you sure you want to quit?").centered(),
                    msg_area,
                );

                let buf = frame.buffer_mut();
                self.buttons.render(buf, keys_area, p, viewport);
            });
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;

    use super::QuitModal;
    use crate::action::Action;

    #[test]
    fn new_creates_modal() {
        let _modal = QuitModal::new();
    }

    #[test]
    fn handle_key_yes_on_y() {
        let mut modal = QuitModal::new();
        assert_eq!(modal.handle_key(KeyCode::Char('y')), Some(Action::Quit));
    }

    #[test]
    fn handle_key_dismiss_on_n() {
        let mut modal = QuitModal::new();
        assert_eq!(modal.handle_key(KeyCode::Char('n')), Some(Action::DismissQuit));
    }

    #[test]
    fn handle_key_dismiss_on_esc() {
        let mut modal = QuitModal::new();
        assert_eq!(modal.handle_key(KeyCode::Esc), Some(Action::DismissQuit));
    }

    #[test]
    fn handle_key_enter_on_focused_yes() {
        let mut modal = QuitModal::new();
        // Yes (index 0) is focused by default
        assert_eq!(modal.handle_key(KeyCode::Enter), Some(Action::Quit));
    }

    #[test]
    fn handle_key_enter_on_focused_no() {
        let mut modal = QuitModal::new();
        // Tab to No
        modal.handle_key(KeyCode::Tab);
        assert_eq!(modal.handle_key(KeyCode::Enter), Some(Action::DismissQuit));
    }

    #[test]
    fn handle_key_none_for_other_keys() {
        let mut modal = QuitModal::new();
        assert_eq!(modal.handle_key(KeyCode::Char('a')), None);
        assert_eq!(modal.handle_key(KeyCode::Up), None);
        assert_eq!(modal.handle_key(KeyCode::Char('j')), None);
    }

    #[test]
    fn tab_cycles_focus() {
        let mut modal = QuitModal::new();
        // Tab to No
        assert_eq!(modal.handle_key(KeyCode::Tab), None);
        assert_eq!(modal.handle_key(KeyCode::Enter), Some(Action::DismissQuit));
        // Tab back to Yes
        assert_eq!(modal.handle_key(KeyCode::Tab), None);
        assert_eq!(modal.handle_key(KeyCode::Enter), Some(Action::Quit));
    }
}
