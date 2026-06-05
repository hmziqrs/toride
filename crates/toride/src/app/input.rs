//! Input handling: keyboard and mouse dispatch.
//!
//! Routes key and mouse events to the active screen via the [`AppScreen`]
//! trait, returning an [`Action`] when the screen requests navigation or quit.
//! When a modal is open, all input is intercepted by the modal.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};

use crate::action::Action;
use crate::navigation::Screen;
use crate::ui::screens::help::HelpScreen;
use crate::ui::widgets::ModalEvent;

use super::App;

impl App {
    /// Handle a keyboard event, returning an [`Action`] if navigation is requested.
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        // Quit confirmation modal intercepts all input when visible
        if self.quit_visible {
            return self.quit_modal.handle_key(key.code);
        }

        // Help modal intercepts all input when visible.
        // Domain-specific keys (q, b, ?) handled first, then InteractiveModal.
        if self.help_modal.is_visible() {
            match key.code {
                KeyCode::Char('q') => return Some(Action::Quit),
                KeyCode::Char('b') | KeyCode::Char('?') | KeyCode::Esc => {
                    self.help_modal.close();
                    return Some(Action::CloseHelp);
                }
                other => {
                    self.help_modal.handle_key(other);
                    return None;
                }
            }
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
            return self.quit_modal.handle_mouse(&mouse);
        }

        // Help modal: InteractiveModal handles click-outside.
        if self.help_modal.is_visible() {
            return match self.help_modal.handle_mouse(&mouse) {
                ModalEvent::Closed => {
                    self.needs_redraw = true;
                    Some(Action::CloseHelp)
                }
                _ => None,
            };
        }

        if self.transition.is_some() {
            return None;
        }

        self.current_screen().handle_mouse(mouse)
    }
}
