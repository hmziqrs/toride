//! Quit confirmation modal.
//!
//! A self-contained modal that asks the user to confirm before quitting.
//! Owns its own interactive buttons, focus state, rendering, and input handling.

use crossterm::event::{KeyCode, MouseEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::Paragraph,
};
use ratatui_interact::state::FocusManager;

use crate::action::Action;
use crate::ui::components::interactive_button::InteractiveButton;
use crate::ui::responsive::Viewport;
use crate::ui::theme::Palette;
use crate::ui::widgets::Modal;

/// Button index mapping.
const IDX_YES: usize = 0;
const IDX_NO: usize = 1;

/// Actions for each button.
const BTN_ACTIONS: &[Action] = &[Action::Quit, Action::DismissQuit];

/// Horizontal gap between the two buttons.
const BTN_GAP: u16 = 4;

/// Self-contained quit confirmation modal.
///
/// Owns interactive Yes/No buttons with focus cycling and mouse support.
/// Delegates rendering to the shared [`Modal`] widget.
pub struct QuitModal {
    buttons: [InteractiveButton<Action>; 2],
    focus: FocusManager<usize>,
}

impl QuitModal {
    #[must_use]
    pub fn new() -> Self {
        let buttons = [
            InteractiveButton::new("yes", "y", Action::Quit),
            InteractiveButton::new("no", "n", Action::DismissQuit),
        ];
        let mut focus = FocusManager::new();
        focus.register_all([IDX_YES, IDX_NO]);

        let mut modal = Self { buttons, focus };
        modal.sync_focus();
        modal
    }

    /// Handle a key press while the quit modal is open.
    pub fn handle_key(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            // Direct shortcuts
            KeyCode::Char('y') => Some(Action::Quit),
            KeyCode::Char('n') | KeyCode::Esc => Some(Action::DismissQuit),
            // Enter activates the focused button
            KeyCode::Enter => {
                let action = match self.focus.current() {
                    Some(&idx) => BTN_ACTIONS[idx],
                    None => Action::Quit,
                };
                Some(action)
            }
            // Focus cycling
            KeyCode::Tab | KeyCode::Right => {
                self.focus.next();
                self.sync_focus();
                None
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.focus.prev();
                self.sync_focus();
                None
            }
            _ => None,
        }
    }

    /// Handle a mouse event while the quit modal is open.
    pub fn handle_mouse(&mut self, mouse: &MouseEvent) -> Option<Action> {
        self.buttons
            .iter_mut()
            .find_map(|btn| btn.handle_mouse(mouse))
    }

    /// Render the quit modal overlay.
    pub fn render(&mut self, frame: &mut ratatui::Frame, p: Palette) {
        self.sync_focus();
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

                // Interactive Yes / No buttons
                let widths: [u16; 2] = std::array::from_fn(|i| self.buttons[i].min_width(viewport));
                let total_w = widths[0] + BTN_GAP + widths[1];
                let mut cx = keys_area.x + (keys_area.width.saturating_sub(total_w)) / 2;

                let buf = frame.buffer_mut();
                for (i, &w) in widths.iter().enumerate() {
                    let btn_area = Rect::new(cx, keys_area.y, w, 1);
                    self.buttons[i].render(buf, btn_area, p, viewport);
                    cx += w + BTN_GAP;
                }
            });
    }

    fn sync_focus(&mut self) {
        let focused = self.focus.current().copied();
        for (i, btn) in self.buttons.iter_mut().enumerate() {
            btn.set_focused(focused == Some(i));
        }
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
