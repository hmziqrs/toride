//! Input handling: keyboard and mouse dispatch.
//!
//! Routes key and mouse events to the active screen via the [`AppScreen`]
//! trait, returning an [`Action`] when the screen requests navigation or quit.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};

use crate::action::Action;

use super::App;

impl App {
    /// Handle a keyboard event, returning an [`Action`] if navigation is requested.
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
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

        self.current_screen().handle_key(key.code)
    }

    /// Handle a mouse event, returning an [`Action`] if navigation is requested.
    pub(super) fn handle_mouse(&mut self, mouse: MouseEvent) -> Option<Action> {
        if self.transition.is_some() {
            return None;
        }

        self.current_screen().handle_mouse(mouse)
    }
}
