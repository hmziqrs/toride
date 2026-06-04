//! Input handling: keyboard and mouse dispatch.
//!
//! Routes key and mouse events to the active screen via the [`AppScreen`]
//! trait, returning an [`Action`] when the screen requests navigation or quit.
//! When a modal is open, all input is intercepted by the modal.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};

use crate::action::Action;
use crate::navigation::Screen;
use crate::ui::screens::help::HelpScreen;

use super::App;

/// Actions for the quit modal buttons (index 0 = Yes, 1 = No).
const QUIT_BTN_ACTIONS: &[Action] = &[Action::Quit, Action::DismissQuit];

impl App {
    /// Handle a keyboard event, returning an [`Action`] if navigation is requested.
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        // Quit confirmation modal intercepts all input when visible
        if self.quit_visible {
            match key.code {
                // Direct shortcuts
                KeyCode::Char('y') => return Some(Action::Quit),
                KeyCode::Char('n') | KeyCode::Esc => return Some(Action::DismissQuit),
                // Enter activates the focused button
                KeyCode::Enter => {
                    let action = match self.quit_focus.current() {
                        Some(&idx) => QUIT_BTN_ACTIONS[idx],
                        None => Action::Quit,
                    };
                    return Some(action);
                }
                // Focus cycling
                KeyCode::Tab | KeyCode::Right => self.quit_focus.next(),
                KeyCode::BackTab | KeyCode::Left => self.quit_focus.prev(),
                _ => {}
            }
            return None;
        }

        // Help modal intercepts all input when visible
        if self.help_visible {
            return HelpScreen::handle_key(key.code);
        }

        if self.transition.is_some() {
            return None;
        }

        // Global keybindings — work on every screen
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            if let KeyCode::Char('t') = key.code {
                return Some(Action::CycleTheme);
            }
            // Don't forward Ctrl+other to screens
            return None;
        }

        // Global `?` opens help from any screen
        if key.code == KeyCode::Char('?') {
            return Some(Action::Help);
        }

        // On welcome screen, `q` quits immediately.
        // On all other screens, `q` shows the confirmation modal.
        if key.code == KeyCode::Char('q') && self.nav.current() != Screen::Welcome {
            return Some(Action::ConfirmQuit);
        }

        self.current_screen().handle_key(key.code)
    }

    /// Handle a mouse event, returning an [`Action`] if navigation is requested.
    pub(super) fn handle_mouse(&mut self, mouse: MouseEvent) -> Option<Action> {
        // Quit modal buttons get first dibs on mouse events
        if self.quit_visible {
            return self
                .quit_buttons
                .iter_mut()
                .find_map(|btn| btn.handle_mouse(&mouse));
        }

        // Swallow all mouse events while help modal is open
        if self.help_visible {
            return None;
        }

        if self.transition.is_some() {
            return None;
        }

        self.current_screen().handle_mouse(mouse)
    }
}
