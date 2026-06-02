//! Screen routing and navigation back-stack.
//!
//! [`Navigator`] owns the current screen and navigation history, providing
//! forward and backward navigation with animated transitions.

use crate::ui::transition::{TransitionCache, TransitionState};

// ── Screen enum ─────────────────────────────────────────────────────────────

/// Identifies a screen in the application.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum Screen {
    /// Welcome / splash screen.
    #[default]
    Welcome = 0,
    /// System status dashboard.
    Status = 1,
    /// Help / keybindings screen.
    Help = 2,
}

impl Screen {
    /// Compact numeric key used by [`TransitionCache`].
    pub(crate) fn key(self) -> u8 {
        self as u8
    }

    /// Convert a numeric key back to a [`Screen`].
    ///
    /// Returns [`Screen::Welcome`] for unknown keys.
    #[expect(
        clippy::wildcard_in_or_patterns,
        clippy::match_same_arms,
        reason = "fallback for unknown screen keys"
    )]
    pub(crate) fn from_key(key: u8) -> Self {
        match key {
            0 => Screen::Welcome,
            1 => Screen::Status,
            2 => Screen::Help,
            _ => Screen::Welcome,
        }
    }
}

// ── Navigator ───────────────────────────────────────────────────────────────

/// Manages the current screen and navigation back-stack.
pub struct Navigator {
    screen: Screen,
    nav_stack: Vec<Screen>,
}

impl Navigator {
    /// Create a navigator starting at the welcome screen.
    #[must_use]
    pub fn new() -> Self {
        Self {
            screen: Screen::Welcome,
            nav_stack: vec![Screen::Welcome],
        }
    }

    /// Current active screen.
    pub fn current(&self) -> Screen {
        self.screen
    }

    /// Whether the navigator can go back (more than one entry in the stack).
    pub fn can_go_back(&self) -> bool {
        self.nav_stack.len() > 1
    }

    /// Begin a forward navigation to `target`.
    ///
    /// Returns a [`TransitionState`] for the animated transition.
    /// Call [`commit_forward`](Self::commit_forward) when the transition completes.
    pub fn start_forward(
        &self,
        target: Screen,
        cache: &mut TransitionCache,
    ) -> TransitionState {
        TransitionState::new(self.screen.key(), target.key(), cache, false)
    }

    /// Begin a backward navigation.
    ///
    /// Returns `None` if there is nowhere to go back to.
    /// Call [`commit_back`](Self::commit_back) when the transition completes.
    pub fn start_backward(&mut self, cache: &mut TransitionCache) -> Option<TransitionState> {
        if self.nav_stack.len() <= 1 {
            return None;
        }
        let from = self.screen;
        // Pop the current screen now so the stack is ready when transition completes
        self.nav_stack.pop();
        let Some(&target) = self.nav_stack.last() else {
            return None;
        };
        Some(TransitionState::new(from.key(), target.key(), cache, true))
    }

    /// Finalize a completed forward transition.
    pub fn commit_forward(&mut self, target: Screen) {
        self.screen = target;
        self.nav_stack.push(target);
    }

    /// Finalize a completed backward transition.
    pub fn commit_back(&mut self, target: Screen) {
        self.screen = target;
        // Stack was already popped in start_backward
    }
}

impl Default for Navigator {
    fn default() -> Self {
        Self::new()
    }
}
