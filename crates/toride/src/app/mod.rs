//! Application state, event loop, and update logic.
//!
//! The [`App`] struct is the top-level orchestrator that owns all screen
//! instances, navigation state, and drives the main event loop via tokio's
//! `select!`.

mod input;
mod render;

use color_eyre::eyre::Result;
use crossterm::event::{Event, EventStream, KeyEventKind};
use futures::StreamExt;
use ratatui::DefaultTerminal;
use tokio::select;

use crate::action::Action;
use crate::navigation::{Navigator, Screen};
use crate::status_collector::StatusCollector;
use crate::ui::screens::help::HelpScreen;
use crate::ui::screens::status::StatusScreen;
use crate::ui::screens::welcome::WelcomeScreen;
use crate::ui::transition::{TransitionCache, TransitionState};

/// Top-level application orchestrator.
///
/// Owns all screen instances, the navigation state, and drives the main
/// event loop via tokio's `select!`.
pub struct App {
    nav: Navigator,
    welcome: WelcomeScreen,
    status: StatusScreen,
    help: HelpScreen,
    should_quit: bool,
    transition: Option<TransitionState>,
    transition_cache: TransitionCache,
    collector: StatusCollector,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Create a new application starting at the welcome screen.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nav: Navigator::new(),
            welcome: WelcomeScreen::new(),
            status: StatusScreen::new(),
            help: HelpScreen::new(),
            should_quit: false,
            transition: None,
            transition_cache: TransitionCache::new(),
            collector: StatusCollector::new(),
        }
    }

    fn update(&mut self, action: Action) {
        if self.transition.is_some() {
            return;
        }

        match action {
            Action::Quit => self.should_quit = true,
            Action::Continue => self.start_forward(Screen::Status),
            Action::Help => self.start_forward(Screen::Help),
            Action::Back => self.go_back(),
        }
    }

    fn start_forward(&mut self, to: Screen) {
        let state = self.nav.start_forward(to, &mut self.transition_cache);
        self.transition = Some(state);
    }

    fn go_back(&mut self) {
        if let Some(state) = self.nav.start_backward(&mut self.transition_cache) {
            self.transition = Some(state);
        }
    }

    /// Run the main event loop.
    ///
    /// # Errors
    ///
    /// Returns an error if the terminal draw fails or the event stream encounters an error.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let mut events = EventStream::new();
        let refresh_interval = tokio::time::interval(std::time::Duration::from_secs(2));
        let anim_tick = tokio::time::interval(std::time::Duration::from_millis(33)); // ~30fps
        tokio::pin!(refresh_interval);
        tokio::pin!(anim_tick);

        loop {
            terminal.draw(|f| self.view(f))?;

            select! {
                // Prioritize terminal events and status results over timer
                biased;

                Some(Ok(event)) = events.next() => {
                    let action = match event {
                        Event::Key(key) if key.kind == KeyEventKind::Press => {
                            self.handle_key(key.code)
                        }
                        Event::Mouse(mouse) => self.handle_mouse(mouse),
                        Event::Resize(..) => {
                            self.welcome.invalidate_cache();
                            self.status.invalidate_cache();
                            self.help.invalidate_cache();
                            None
                        }
                        _ => None,
                    };
                    if let Some(action) = action {
                        self.update(action);
                    }
                }

                // Receive collected status data
                Some(status) = self.collector.poll(), if self.collector.is_pending() => {
                    self.status.set_status(status);
                }

                // Periodic status refresh
                _ = refresh_interval.tick() => {
                    if matches!(self.nav.current(), Screen::Status) {
                        self.collector.start();
                    }
                }

                // Animation tick (~30fps for shimmer, border, spinner, and transitions)
                _ = anim_tick.tick(),
                    if self.transition.is_some()
                        || matches!(self.nav.current(), Screen::Welcome | Screen::Status) => {}
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }
}
