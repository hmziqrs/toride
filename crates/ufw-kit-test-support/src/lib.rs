//! Test support utilities for ufw-kit.
//!
//! Re-exports the fake command runner and provides test fixtures
//! for writing integration tests against ufw-kit.

pub use ufw_kit::command::{CommandLog, CommandRunner, FakeRunner};
pub use ufw_kit::spec::{CommandResult, CommandSpec};
