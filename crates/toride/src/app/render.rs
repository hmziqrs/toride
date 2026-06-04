//! Transition-aware view rendering.
//!
//! Handles the main render dispatch, screen transitions (animated gradient
//! swaps at the midpoint), per-screen cache invalidation, and the help modal
//! overlay.

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    widgets::Paragraph,
};
use ratatui::style::Stylize;
use tachyonfx::Interpolation;

use crate::navigation::Screen;
use crate::ui::responsive::Viewport;
use crate::ui::screens::help::HelpScreen;
use crate::ui::widgets::Modal;

use super::App;

impl App {
    /// Main render method. Handles transition rendering and delegates to the
    /// active screen's `view()`.
    pub(super) fn view(&mut self, frame: &mut Frame) {
        let p = self.active_theme.palette();

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
                *p,
                offset,
                ts.params.edge_delta,
                ts.params.brightness_dip,
                eased,
            );

            // Render foreground of appropriate screen
            if show_to {
                let to_screen = Screen::from_key(ts.to);
                self.screen_by_enum(to_screen).view_foreground(frame, *p);
            } else {
                self.current_screen().view_foreground(frame, *p);
            }

            // Check completion — reconstitute transition only if not done
            if ts.is_done() {
                let to_screen = Screen::from_key(ts.to);
                if ts.reverse {
                    self.nav.commit_back(to_screen);
                } else {
                    self.nav.commit_forward(to_screen);
                }
                self.screen_by_enum(to_screen).invalidate_cache();
                self.transition = None;
            } else {
                self.transition = Some(ts);
            }
        } else {
            self.current_screen().view(frame, *p);
        }

        // Help modal overlay — rendered on top of whatever is behind it
        if self.help_visible {
            let viewport = Viewport::from_area(frame.area());
            Modal::new("Help").render(frame, *p, |frame, content_area| {
                HelpScreen::render(frame, content_area, *p, viewport);
            });
        }

        // Quit confirmation modal
        if self.quit_visible {
            // Sync focus state to buttons
            let focused = self.quit_focus.current().copied();
            for (i, btn) in self.quit_buttons.iter_mut().enumerate() {
                btn.set_focused(focused == Some(i));
            }

            let viewport = Viewport::from_area(frame.area());
            let quit_buttons = &mut self.quit_buttons;
            Modal::new("Quit?")
                .dimensions(36, 7)
                .render(frame, *p, |frame, content_area| {
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

                    // Render interactive Yes / No buttons
                    let gap: u16 = 4;
                    let widths: [u16; 2] =
                        std::array::from_fn(|i| quit_buttons[i].min_width(viewport));
                    let total_w = widths[0] + gap + widths[1];
                    let mut cx = keys_area.x + (keys_area.width.saturating_sub(total_w)) / 2;

                    let buf = frame.buffer_mut();
                    for (i, &w) in widths.iter().enumerate() {
                        let btn_area = ratatui::layout::Rect::new(cx, keys_area.y, w, 1);
                        quit_buttons[i].render(buf, btn_area, *p, viewport);
                        cx += w + gap;
                    }
                });
        }
    }

    /// Get a specific screen by its [`Screen`] enum value as `&mut dyn AppScreen`.
    ///
    /// This is needed during transitions where we must address a screen that
    /// may not be the *current* one.
    fn screen_by_enum(&mut self, screen: Screen) -> &mut dyn crate::ui::screens::AppScreen {
        match screen {
            Screen::Welcome => &mut self.welcome,
            Screen::Dashboard => &mut self.dashboard,
        }
    }
}
