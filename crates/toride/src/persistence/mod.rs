//! State persistence across sessions.
//!
//! This module will handle saving and restoring application state so that
//! user preferences and session data survive restarts.
//!
//! Planned features:
//! - Last-used theme and screen
//! - Scroll position memory
//! - Custom keybinding profiles
//!
//! Planned dependencies: `serde_json`, `dirs`.
