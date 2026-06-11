//! Application state, event loop, and update logic.
//!
//! The [`App`] struct is the top-level orchestrator that owns all screen
//! instances, navigation state, and drives the main event loop via tokio's
//! `select!`.

mod input;
mod render;

use std::time::Instant;

use color_eyre::eyre::Result;
use crossterm::event::{Event, EventStream, KeyEventKind};
use futures::StreamExt;
use ratatui::DefaultTerminal;
use tokio::select;
use tokio::sync::mpsc;

use crate::action::Action;
use crate::navigation::{Navigator, Screen};
use crate::ssh_data::{SshDataCollector, execute_op};
use crate::status_collector::StatusCollector;
use crate::ui::screens::AppScreen;
use crate::ui::screens::help::HelpScreen;
use crate::ui::screens::quit::QuitModal;
use crate::ui::screens::dashboard::DashboardScreen;
use crate::ui::screens::welcome::WelcomeScreen;
use crate::ui::theme::Theme;
use crate::ui::transition::{TransitionCache, TransitionState};
use crate::ui::widgets::InteractiveModal;

/// Top-level application orchestrator.
///
/// Owns all screen instances, the navigation state, and drives the main
/// event loop via tokio's `select!`.
pub struct App {
    nav: Navigator,
    welcome: WelcomeScreen,
    dashboard: DashboardScreen,
    help: HelpScreen,
    /// Interactive help modal (manages visibility + rect + click-outside).
    help_modal: InteractiveModal<Action>,
    quit_visible: bool,
    quit_modal: QuitModal,
    active_theme: Theme,
    should_quit: bool,
    needs_redraw: bool,
    transition: Option<TransitionState>,
    transition_cache: TransitionCache,
    collector: StatusCollector,
    ssh_collector: SshDataCollector,
    /// Receiver for SSH write operation error messages.
    ssh_error_rx: mpsc::UnboundedReceiver<String>,
    /// Sender clone passed to spawned SSH write tasks.
    ssh_error_tx: mpsc::UnboundedSender<String>,
    /// Receiver for SSH write operation completion signals.
    ssh_op_done_rx: mpsc::UnboundedReceiver<()>,
    /// Sender clone passed to spawned SSH write tasks (signals completion).
    ssh_op_done_tx: mpsc::UnboundedSender<()>,
    /// Number of SSH write ops currently in-flight.
    ssh_ops_in_flight: usize,
    /// Time of the last SSH write op — suppresses data refresh to avoid
    /// overwriting optimistic in-memory updates before the async write lands.
    ssh_write_cooldown: Option<Instant>,
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
        let (ssh_error_tx, ssh_error_rx) = mpsc::unbounded_channel();
        let (ssh_op_done_tx, ssh_op_done_rx) = mpsc::unbounded_channel();
        Self {
            ssh_error_tx,
            ssh_error_rx,
            ssh_op_done_tx,
            ssh_op_done_rx,
            ssh_ops_in_flight: 0,
            nav: Navigator::new(),
            welcome: WelcomeScreen::new(),
            dashboard: DashboardScreen::new(),
            help: HelpScreen::new(),
            help_modal: InteractiveModal::display("Help").dimensions(52, 16),
            quit_visible: false,
            quit_modal: QuitModal::new(),
            active_theme: Theme::default(),
            should_quit: false,
            needs_redraw: false,
            transition: None,
            transition_cache: TransitionCache::new(),
            collector: StatusCollector::new(),
            ssh_collector: SshDataCollector::new(),
            ssh_write_cooldown: None,
        }
    }

    /// Return a mutable reference to the current screen as `dyn AppScreen`.
    fn current_screen(&mut self) -> &mut dyn AppScreen {
        self.screen_by_enum(self.nav.current())
    }

    /// Invalidate all screen caches and flag a full redraw.
    fn invalidate_all_caches(&mut self) {
        self.welcome.invalidate_cache();
        self.dashboard.invalidate_cache();
        self.needs_redraw = true;
    }

    fn update(&mut self, action: Action) {
        if self.transition.is_some() {
            return;
        }

        match action {
            Action::Quit => self.should_quit = true,
            Action::ConfirmQuit => {
                self.quit_visible = true;
                self.needs_redraw = true;
            }
            Action::DismissQuit => {
                self.quit_visible = false;
                self.needs_redraw = true;
            }
            Action::Continue => self.start_forward(Screen::Dashboard),
            Action::Help => {
                if self.help_modal.is_visible() {
                    self.help_modal.close();
                } else {
                    self.help_modal.open();
                }
                self.needs_redraw = true;
            }
            Action::CloseHelp => {
                self.help_modal.close();
                self.needs_redraw = true;
            }
            Action::Back => self.go_back(),
            Action::CycleTheme => {
                let all = Theme::all();
                let idx = all
                    .iter()
                    .position(|&t| t == self.active_theme)
                    .unwrap_or(0);
                let next = all[(idx + 1) % all.len()];
                self.active_theme = next;
                self.welcome.set_border_color(next.palette().accent);
                self.invalidate_all_caches();
            }
            // Scroll actions (and any future screen-local actions) are routed
            // to the current screen via `handle_action`.
            _ => self.current_screen().handle_action(action),
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

    /// Drain pending SSH write operations and spawn async tasks for each.
    /// Errors are sent back via the error channel and surfaced in the UI.
    /// Sets a 5-second cooldown on SSH data refresh to prevent the next
    /// refresh from overwriting optimistic in-memory updates before the
    /// async write lands on disk.
    fn flush_ssh_ops(&mut self) {
        if !matches!(self.nav.current(), Screen::Dashboard) {
            return;
        }
        let tx = self.ssh_error_tx.clone();
        let done_tx = self.ssh_op_done_tx.clone();
        let ops = self.dashboard.drain_ssh_ops();
        if ops.is_empty() {
            return;
        }
        // Set cooldown: skip SSH data refresh for 5 seconds to avoid
        // clobbering the optimistic in-memory update with stale disk state.
        self.ssh_write_cooldown = Some(Instant::now());
        self.ssh_ops_in_flight += ops.len();
        self.dashboard.set_ssh_loading(true, self.ssh_ops_in_flight);
        for op in ops {
            let tx = tx.clone();
            let done_tx = done_tx.clone();
            tokio::spawn(async move {
                if let Err(msg) = execute_op(op).await {
                    let _ = tx.send(msg);
                }
                let _ = done_tx.send(());
            });
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
            self.needs_redraw = false;

            select! {
                // Prioritize terminal events and status results over timer
                biased;

                Some(Ok(event)) = events.next() => {
                    let action = match event {
                        Event::Key(key) if key.kind == KeyEventKind::Press => {
                            self.handle_key(key)
                        }
                        Event::Mouse(mouse) => self.handle_mouse(mouse),
                        Event::Resize(..) => {
                            self.invalidate_all_caches();
                            None
                        }
                        _ => None,
                    };
                    self.flush_ssh_ops();
                    if let Some(action) = action {
                        self.update(action);
                        self.needs_redraw = true;
                    }
                }

                // Receive collected status data
                Some(status) = self.collector.poll(), if self.collector.is_pending() => {
                    self.dashboard.set_status(status);
                    self.needs_redraw = true;
                }

                // Receive collected SSH data
                Some(bundle) = self.ssh_collector.poll(), if self.ssh_collector.is_pending() => {
                    self.dashboard.set_ssh_data(bundle);
                    self.needs_redraw = true;
                }

                // Receive SSH write errors from spawned tasks
                Some(msg) = self.ssh_error_rx.recv() => {
                    if matches!(self.nav.current(), Screen::Dashboard) {
                        self.dashboard.push_ssh_error(msg);
                        self.needs_redraw = true;
                    }
                }

                // Receive SSH write op completion signals
                Some(()) = self.ssh_op_done_rx.recv() => {
                    self.ssh_ops_in_flight = self.ssh_ops_in_flight.saturating_sub(1);
                    let loading = self.ssh_ops_in_flight > 0;
                    self.dashboard.set_ssh_loading(loading, self.ssh_ops_in_flight);
                    self.needs_redraw = true;
                }

                // Periodic status refresh
                _ = refresh_interval.tick() => {
                    if matches!(self.nav.current(), Screen::Dashboard) {
                        self.dashboard.tick_clock();
                        self.collector.start();
                        // Skip SSH data refresh during write cooldown to prevent
                        // overwriting optimistic in-memory updates with stale
                        // disk state. Cooldown expires after 5 seconds.
                        let skip_ssh = self.ssh_write_cooldown
                            .is_some_and(|t| t.elapsed().as_secs() < 5);
                        if !skip_ssh {
                            self.ssh_write_cooldown = None;
                            self.ssh_collector.start();
                        }
                        self.needs_redraw = true;
                    }
                }

                // Animation tick (~30fps for shimmer, border, spinner, and transitions)
                _ = anim_tick.tick(),
                    if self.transition.is_some()
                        || self.needs_redraw
                        || matches!(self.nav.current(), Screen::Welcome | Screen::Dashboard) => {}
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::action::Action;
    use crate::app::App;
    use crate::navigation::Screen;
    use crate::ui::theme::Theme;

    #[test]
    fn new_creates_default_state() {
        let app = App::new();
        assert_eq!(app.active_theme, Theme::Charm);
        assert!(!app.should_quit);
        assert_eq!(app.nav.current(), Screen::Welcome);
    }

    #[test]
    fn default_equals_new() {
        let from_new = App::new();
        let from_default = App::default();
        assert_eq!(from_new.active_theme, from_default.active_theme);
        assert_eq!(from_new.should_quit, from_default.should_quit);
        assert_eq!(from_new.nav.current(), from_default.nav.current());
        assert!(from_new.transition.is_none());
        assert!(from_default.transition.is_none());
    }

    #[test]
    fn update_quit_sets_should_quit() {
        let mut app = App::new();
        assert!(!app.should_quit);
        app.update(Action::Quit);
        assert!(app.should_quit);
    }

    #[test]
    fn update_continue_starts_transition_to_status() {
        let mut app = App::new();
        assert!(app.transition.is_none());
        app.update(Action::Continue);
        assert!(app.transition.is_some());
    }

    #[test]
    fn update_help_toggles_modal() {
        let mut app = App::new();
        assert!(!app.help_modal.is_visible());
        app.update(Action::Help);
        assert!(app.help_modal.is_visible());
        app.update(Action::Help);
        assert!(!app.help_modal.is_visible());
    }

    #[test]
    fn update_close_help_hides_modal() {
        let mut app = App::new();
        app.help_modal.open();
        app.update(Action::CloseHelp);
        assert!(!app.help_modal.is_visible());
    }

    #[test]
    fn update_back_does_nothing_at_welcome() {
        let mut app = App::new();
        assert!(app.transition.is_none());
        app.update(Action::Back);
        assert!(app.transition.is_none());
        assert_eq!(app.nav.current(), Screen::Welcome);
        assert!(!app.should_quit);
    }

    #[test]
    fn update_confirm_quit_shows_modal() {
        let mut app = App::new();
        assert!(!app.quit_visible);
        app.update(Action::ConfirmQuit);
        assert!(app.quit_visible);
    }

    #[test]
    fn update_dismiss_quit_hides_modal() {
        let mut app = App::new();
        app.quit_visible = true;
        app.update(Action::DismissQuit);
        assert!(!app.quit_visible);
    }
}
