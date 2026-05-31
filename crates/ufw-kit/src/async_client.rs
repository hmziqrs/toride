//! Async UFW client wrappers (behind `tokio` feature).
//!
//! Provides async versions of key [`crate::Ufw`] methods using
//! `tokio::task::spawn_blocking` to offload blocking command execution
//! to a background thread pool.

use crate::error::{Error, Result};
use crate::spec::{
    DeleteOptions, Direction, DisableOptions, EnableOptions, LoggingLevel, Policy, ResetOptions,
    RouteRuleSpec, RuleSpec, UfwReport, UfwStatus,
};

/// An async wrapper around [`crate::Ufw`].
///
/// This struct owns a [`crate::Ufw`] instance and exposes async methods
/// that internally use `tokio::task::spawn_blocking` to run blocking
/// command execution on a separate thread.
///
/// # Example
///
/// ```rust,no_run,ignore
/// use ufw_kit::async_client::AsyncUfw;
///
/// #[tokio::main]
/// async fn main() {
///     let ufw = AsyncUfw::system();
///     let status = ufw.status().await.unwrap();
///     println!("UFW is {}", if status.active { "active" } else { "inactive" });
/// }
/// ```
pub struct AsyncUfw {
    inner: std::sync::Arc<crate::Ufw>,
}

impl AsyncUfw {
    /// Create an async UFW client using the real system runner.
    #[must_use]
    pub fn system() -> Self {
        Self {
            inner: std::sync::Arc::new(crate::Ufw::system()),
        }
    }

    /// Create an async UFW client with a custom command runner.
    pub fn with_runner(runner: impl crate::command::CommandRunner + 'static) -> Self {
        Self {
            inner: std::sync::Arc::new(crate::Ufw::with_runner(runner)),
        }
    }

    /// Get UFW version (async).
    pub async fn version(&self) -> Result<String> {
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || ufw.version())
            .await
            .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Get UFW status (async).
    pub async fn status(&self) -> Result<UfwStatus> {
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || ufw.status())
            .await
            .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Get UFW verbose status (async).
    pub async fn status_verbose(&self) -> Result<UfwStatus> {
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || ufw.status_verbose())
            .await
            .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Get UFW numbered status (async).
    pub async fn status_numbered(&self) -> Result<UfwStatus> {
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || ufw.status_numbered())
            .await
            .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Enable UFW (async).
    pub async fn enable(&self, opts: &EnableOptions) -> Result<()> {
        let opts = opts.clone();
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || ufw.enable(&opts))
            .await
            .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Disable UFW (async).
    pub async fn disable(&self, opts: &DisableOptions) -> Result<()> {
        let opts = opts.clone();
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || ufw.disable(&opts))
            .await
            .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Reload UFW (async).
    pub async fn reload(&self) -> Result<()> {
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || ufw.reload())
            .await
            .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Reset UFW (async).
    pub async fn reset(&self, opts: &ResetOptions) -> Result<()> {
        let opts = opts.clone();
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || ufw.reset(&opts))
            .await
            .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Set default policy (async).
    pub async fn set_default_policy(&self, direction: Direction, policy: Policy) -> Result<()> {
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || ufw.set_default_policy(direction, policy))
            .await
            .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Set logging level (async).
    pub async fn set_logging(&self, level: LoggingLevel) -> Result<()> {
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || ufw.set_logging(level))
            .await
            .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Add a rule (async).
    pub async fn add_rule(&self, spec: &RuleSpec) -> Result<()> {
        let spec = spec.clone();
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || ufw.add_rule(&spec))
            .await
            .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Delete a rule (async).
    pub async fn delete_rule(&self, spec: &RuleSpec) -> Result<()> {
        let spec = spec.clone();
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || ufw.delete_rule(&spec))
            .await
            .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Ensure a rule exists idempotently (async).
    pub async fn ensure_rule(&self, spec: &RuleSpec) -> Result<crate::spec::ApplyReport> {
        let spec = spec.clone();
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || ufw.ensure_rule(&spec))
            .await
            .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Add a route rule (async).
    pub async fn add_route_rule(&self, spec: &RouteRuleSpec) -> Result<()> {
        let spec = spec.clone();
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || ufw.add_route_rule(&spec))
            .await
            .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Delete rules by comment (async).
    pub async fn delete_rules_by_comment(&self, comment: &str) -> Result<u32> {
        let comment = comment.to_string();
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || ufw.delete_rules_by_comment(&comment))
            .await
            .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Show a UFW report (async).
    pub async fn show(&self, report: UfwReport) -> Result<String> {
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || ufw.show(report))
            .await
            .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Delete a rule by number (async).
    pub async fn delete_rule_number(&self, number: u32, opts: &DeleteOptions) -> Result<()> {
        let opts = opts.clone();
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || ufw.delete_rule_number(number, &opts))
            .await
            .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Run a dry-run (async).
    pub async fn dry_run(&self, args: &[&str]) -> Result<String> {
        let args: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || {
            let refs: Vec<&str> = args.iter().map(String::as_str).collect();
            ufw.dry_run(&refs)
        })
        .await
        .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Run doctor checks (async).
    #[cfg(feature = "doctor")]
    pub async fn doctor(
        &self,
        scope: crate::spec::DoctorScope,
    ) -> Result<Vec<crate::spec::Finding>> {
        let ufw = self.inner.clone();
        tokio::task::spawn_blocking(move || crate::doctor::doctor(&ufw, scope))
            .await
            .map_err(|e| Error::Other(format!("spawn_blocking failed: {e}")))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn async_ufw_can_be_created() {
        let _ufw = AsyncUfw::system();
    }
}
