#![deny(unsafe_code)]
#![warn(missing_docs)]

//! `toride-diagnostic-types` -- shared diagnostic types for the toride ecosystem.
//!
//! Provides [`Severity`], [`Finding`], [`DoctorReport`], render helpers, and
//! binary/permission check helpers used across `toride-ssh`, `toride-fail2ban`,
//! `ufw-kit`, and the main `toride` binary.
//!
//! # Feature flags
//!
//! | Feature  | Default | Description                           |
//! |----------|---------|---------------------------------------|
//! | `serde`  | no      | `Serialize`/`Deserialize` on types    |

pub mod error;
pub mod finding;
pub mod helpers;
pub mod render;
pub mod report;
pub mod severity;

pub use error::{Error, Result};
pub use finding::Finding;
pub use report::DoctorReport;
pub use severity::Severity;
