//! Transition-aware view rendering.
//!
//! Handles the main render dispatch, screen transitions (animated gradient
//! swaps at the midpoint), and per-screen cache invalidation.

use ratatui::Frame;
use tachyonfx::Interpolation;

use crate::navigation::Screen;

use super::App;

impl App {
    /// Main render method. Handles transition rendering and delegates to the
    /// active screen's `view()`.
    pub(super) fn view(&mut self, frame: &mut Frame) {
        if let Some(ts) = self.transition.take() {
            let raw_progress = ts.progress();
            let eased = Interpolation::CubicInOut.alpha(raw_progress);

            // Determine which screen to show foreground for
            let show_to = if ts.reverse {
                raw_progress > 0.5
            } else {
                raw_progress >= 0.5
            };

            // Render transition gradient
            let area = frame.area();
            let p = crate::ui::theme::CHARM;
            #[expect(
                clippy::cast_lossless,
                reason = "eased is f32 from tachyonfx, offset needs f64"
            )]
            let offset = if ts.reverse {
                (
                    ts.params.center_offset.0 * (1.0 - eased as f64),
                    ts.params.center_offset.1 * (1.0 - eased as f64),
                )
            } else {
                (
                    ts.params.center_offset.0 * eased as f64,
                    ts.params.center_offset.1 * eased as f64,
                )
            };
            crate::ui::widgets::gradient::render_transition_gradient(
                frame.buffer_mut(),
                area,
                p,
                offset,
                ts.params.edge_delta,
                ts.params.brightness_dip,
                eased,
            );

            // Render foreground of appropriate screen
            let current = self.nav.current();
            if show_to {
                let to_screen = Screen::from_key(ts.to);
                self.view_screen_foreground(to_screen, frame);
            } else {
                self.view_screen_foreground(current, frame);
            }

            // Check completion — reconstitute transition only if not done
            if ts.is_done() {
                let to_screen = Screen::from_key(ts.to);
                if ts.reverse {
                    self.nav.commit_back(to_screen);
                } else {
                    self.nav.commit_forward(to_screen);
                }
                self.invalidate_screen_cache(to_screen);
                self.transition = None;
            } else {
                self.transition = Some(ts);
            }
        } else {
            match self.nav.current() {
                Screen::Welcome => self.welcome.view(frame),
                Screen::Status => self.status.view(frame),
                Screen::Help => self.help.view(frame),
            }
        }
    }

    /// Render only the foreground layer of a screen (used during transitions
    /// where the background gradient is already drawn).
    fn view_screen_foreground(&mut self, screen: Screen, frame: &mut Frame) {
        match screen {
            Screen::Welcome => self.welcome.view_foreground(frame),
            Screen::Status => self.status.view_foreground(frame),
            Screen::Help => self.help.view_foreground(frame),
        }
    }

    /// Invalidate the gradient cache for a specific screen.
    fn invalidate_screen_cache(&mut self, screen: Screen) {
        match screen {
            Screen::Welcome => self.welcome.invalidate_cache(),
            Screen::Status => self.status.invalidate_cache(),
            Screen::Help => self.help.invalidate_cache(),
        }
    }
}
