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

impl App {
    /// Handle a keyboard event, returning an [`Action`] if navigation is requested.
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        // Quit confirmation modal intercepts all input when visible
        if self.quit_visible {
            return self.quit_modal.handle_key(key.code);
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
            return self.quit_modal.handle_mouse(&mouse);
        }

        // Help modal: close on click outside, swallow everything else.
        if self.help_visible {
            if let Some(mr) = self.help_modal_rect {
                if let crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) = mouse.kind {
                    let col = mouse.column;
                    let row = mouse.row;
                    if col < mr.x || col >= mr.right() || row < mr.y || row >= mr.bottom() {
                        self.help_visible = false;
                        self.help_modal_rect = None;
                        self.needs_redraw = true;
                    }
                }
            }
            return None;
        }

        if self.transition.is_some() {
            return None;
        }

        self.current_screen().handle_mouse(mouse)
    }
}
