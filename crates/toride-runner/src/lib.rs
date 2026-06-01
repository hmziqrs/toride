//! # toride-runner
//!
//! Shared command runner trait, implementations, and utilities.
//!
//! This crate extracts the common `Runner` trait, `CommandSpec`, `CommandOutput`,
//! and related types from duplicated patterns across the workspace. It provides:
//!
//! - A sync [`Runner`] trait for executing commands
//! - [`CommandSpec`] for describing commands to run
//! - [`CommandOutput`] for capturing results
//! - Argument redaction for sensitive flags
//! - Binary discovery helpers
//! - A real implementation via `duct` (feature `duct-runner`)
//! - A fake implementation for testing (feature `fake`)
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use toride_runner::{CommandSpec, DuctRunner, Runner};
//!
//! let runner = DuctRunner;
//! let spec = CommandSpec::new("echo").arg("hello");
//! let output = runner.run(&spec)?;
//! assert!(output.success);
//! ```

#![deny(unsafe_code)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::module_name_repetitions)]

pub mod discovery;
pub mod error;
pub mod output;
pub mod redact;
pub mod runner;
pub mod spec;

#[cfg(feature = "duct-runner")]
pub mod duct_runner;

#[cfg(feature = "fake")]
pub mod fake;

// Re-export primary types at crate root.
#[cfg(feature = "duct-runner")]
pub use duct_runner::DuctRunner;
pub use error::{Error, Result};
pub use output::CommandOutput;
#[cfg(feature = "fake")]
pub use fake::FakeRunner;
pub use runner::Runner;
pub use spec::CommandSpec;
