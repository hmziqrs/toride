//! User actions that drive the application's [`update`](crate::app::App::update) loop.

/// Semantic actions produced by screens and consumed by [`App::update`](crate::app::App::update).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Proceed to the next screen (Welcome → Status).
    Continue,
    /// Open the help screen.
    Help,
    /// Navigate back to the previous screen.
    Back,
    /// Quit the application.
    Quit,
}
