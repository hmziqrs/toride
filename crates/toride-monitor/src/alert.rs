//! Alert dispatching for anomaly findings.
//!
//! Provides [`AlertDispatcher`] which routes anomaly alerts to configured
//! targets: journald, webhooks, and log files.

use crate::paths::MonitorPaths;
use crate::report::{AlertReport, AnomalyFinding};
use crate::spec::AlertTarget;

/// Dispatches anomaly alerts to configured targets.
///
/// Supports journald, webhook, and file-based alert targets. The dispatcher
/// is designed to be resilient: if one target fails, others are still tried.
pub struct AlertDispatcher<'a> {
    /// Binary paths for system commands.
    paths: &'a MonitorPaths,
}

impl<'a> AlertDispatcher<'a> {
    /// Create a new `AlertDispatcher` with the given paths.
    #[must_use]
    pub fn new(paths: &'a MonitorPaths) -> Self {
        Self { paths }
    }

    /// Dispatch an anomaly finding to all configured alert targets.
    ///
    /// Each target is tried independently. Returns a report for each
    /// dispatch attempt, including any failures.
    #[cfg(feature = "client")]
    pub fn dispatch(&self, finding: &AnomalyFinding, targets: &[AlertTarget]) -> Vec<AlertReport> {
        let mut reports = Vec::new();

        for target in targets {
            let report = match target {
                AlertTarget::Journald { priority } => {
                    self.dispatch_journald(finding, priority)
                }
                AlertTarget::Webhook { url, headers } => {
                    self.dispatch_webhook(finding, url, headers)
                }
                AlertTarget::File { path } => {
                    self.dispatch_file(finding, path)
                }
            };

            reports.push(report);
        }

        reports
    }

    /// Dispatch an alert to the systemd journal via `systemd-cat`.
    #[cfg(feature = "client")]
    fn dispatch_journald(&self, finding: &AnomalyFinding, priority: &str) -> AlertReport {
        let message = format!(
            "[toride-monitor] {} ({}): {} — observed: {}, threshold: {}",
            finding.severity,
            finding.id,
            finding.title,
            finding.observed_value,
            finding.threshold,
        );

        let result = duct::cmd(
            &self.paths.journalctl,
            ["--priority", priority, "--identifier", "toride-monitor"],
        )
        .stdin_bytes(message.as_bytes())
        .stdout_capture()
        .stderr_capture()
        .run();

        match result {
            Ok(output) if output.status.success() => AlertReport {
                finding: finding.clone(),
                target: format!("journald({priority})"),
                dispatched: true,
                error: None,
            },
            Ok(output) => AlertReport {
                finding: finding.clone(),
                target: format!("journald({priority})"),
                dispatched: false,
                error: Some(String::from_utf8_lossy(&output.stderr).into_owned()),
            },
            Err(e) => AlertReport {
                finding: finding.clone(),
                target: format!("journald({priority})"),
                dispatched: false,
                error: Some(format!("{e}")),
            },
        }
    }

    /// Dispatch an alert to a webhook endpoint via HTTP POST.
    fn dispatch_webhook(
        &self,
        finding: &AnomalyFinding,
        url: &str,
        _headers: &[(String, String)],
    ) -> AlertReport {
        // TODO: Implement HTTP POST with proper headers using ureq or similar.
        tracing::warn!(
            "Webhook alert dispatching not yet implemented: {} -> {url}",
            finding.id
        );
        AlertReport {
            finding: finding.clone(),
            target: format!("webhook({url})"),
            dispatched: false,
            error: Some("webhook dispatching not implemented".into()),
        }
    }

    /// Dispatch an alert by appending to a log file.
    fn dispatch_file(&self, finding: &AnomalyFinding, path: &str) -> AlertReport {
        use std::io::Write;

        let line = format!(
            "[{}] {} ({}): {} — observed: {}, threshold: {}",
            finding.severity,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            finding.id,
            finding.title,
            finding.observed_value,
            finding.threshold,
        );

        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            Ok(mut file) => match writeln!(file, "{line}") {
                Ok(()) => AlertReport {
                    finding: finding.clone(),
                    target: format!("file({path})"),
                    dispatched: true,
                    error: None,
                },
                Err(e) => AlertReport {
                    finding: finding.clone(),
                    target: format!("file({path})"),
                    dispatched: false,
                    error: Some(format!("{e}")),
                },
            },
            Err(e) => AlertReport {
                finding: finding.clone(),
                target: format!("file({path})"),
                dispatched: false,
                error: Some(format!("{e}")),
            },
        }
    }
}
