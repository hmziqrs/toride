//! Input handling: keyboard and mouse dispatch.
//!
//! Routes key and mouse events to the active screen, returning an [`Action`]
//! when the screen requests a navigation or quit.

use crossterm::event::{KeyCode, MouseEvent, MouseEventKind};

use crate::action::Action;
use crate::navigation::Screen;

use super::App;

impl App {
    /// Handle a keyboard event, returning an [`Action`] if navigation is requested.
    pub(super) fn handle_key(&mut self, code: KeyCode) -> Option<Action> {
        if self.transition.is_some() {
            return None;
        }
        match self.nav.current() {
            Screen::Welcome => self.welcome.handle_key(code),
            Screen::Status => self.status_handle_key(code),
            Screen::Help => self.help.handle_key(code),
        }
    }

    /// Status-screen-specific key handling with scroll support.
    fn status_handle_key(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.status.scroll_down();
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.status.scroll_up();
                None
            }
            _ => self.status.handle_key(code),
        }
    }

    /// Handle a mouse event, returning an [`Action`] if navigation is requested.
    pub(super) fn handle_mouse(&mut self, mouse: MouseEvent) -> Option<Action> {
        if self.transition.is_some() {
            return None;
        }
        match mouse.kind {
            MouseEventKind::Down(_)
                | MouseEventKind::Up(_)
                | MouseEventKind::Moved
                | MouseEventKind::Drag(..)
                if matches!(self.nav.current(), Screen::Welcome) =>
            {
                self.welcome.handle_mouse(mouse)
            }
            MouseEventKind::ScrollDown if matches!(self.nav.current(), Screen::Status) => {
                self.status.scroll_down();
                None
            }
            MouseEventKind::ScrollUp if matches!(self.nav.current(), Screen::Status) => {
                self.status.scroll_up();
                None
            }
            _ => None,
        }
    }
}
