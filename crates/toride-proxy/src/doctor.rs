//! Diagnostic engine for proxy installations.
//!
//! Provides doctor checks for proxy configuration, security headers,
//! certificate expiry, and service status.

use crate::error::Result;
use crate::parse::{parse_nginx_status, parse_nginx_version};
use crate::paths::ProxyPaths;
use crate::report::{ProxyReport, ProxyStatus};
use toride_runner::{CommandSpec, Runner};

/// Scope for doctor checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoctorScope {
    /// Run all checks.
    All,
    /// Check only proxy service status.
    Service,
    /// Check only security headers.
    Headers,
    /// Check only certificate expiry.
    Certificates,
    /// Check only configuration validity.
    Config,
}

/// A single doctor finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorFinding {
    /// Finding identifier (dot-separated, e.g. "nginx.config-syntax").
    pub id: String,
    /// Severity of the finding.
    pub severity: DoctorSeverity,
    /// Short human-readable title.
    pub title: String,
    /// Longer description.
    pub detail: String,
    /// Suggested fix.
    pub fix: Option<String>,
}

/// Severity level for doctor findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DoctorSeverity {
    /// Informational.
    Info,
    /// Warning.
    Warning,
    /// Error.
    Error,
    /// Critical.
    Critical,
}

impl DoctorFinding {
    /// Create a new finding.
    pub fn new(
        id: impl Into<String>,
        severity: DoctorSeverity,
        title: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            severity,
            title: title.into(),
            detail: String::new(),
            fix: None,
        }
    }

    /// Attach a detail description.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }

    /// Attach a suggested fix.
    pub fn fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(fix.into());
        self
    }
}

/// Diagnostic engine for proxy installations.
pub struct Doctor<'a> {
    runner: &'a dyn Runner,
    paths: &'a ProxyPaths,
}

impl<'a> Doctor<'a> {
    /// Create a new doctor instance.
    pub fn new(runner: &'a dyn Runner, paths: &'a ProxyPaths) -> Self {
        Self { runner, paths }
    }

    /// Run doctor checks for the given scope.
    pub fn run(&self, scope: &DoctorScope) -> Result<ProxyReport> {
        let mut report = ProxyReport::new("nginx");
        let mut findings = Vec::new();

        match scope {
            DoctorScope::All => {
                findings.extend(self.check_service()?);
                findings.extend(self.check_config()?);
                findings.extend(self.check_headers()?);
                findings.extend(self.check_certificates()?);
            }
            DoctorScope::Service => {
                findings.extend(self.check_service()?);
            }
            DoctorScope::Headers => {
                findings.extend(self.check_headers()?);
            }
            DoctorScope::Certificates => {
                findings.extend(self.check_certificates()?);
            }
            DoctorScope::Config => {
                findings.extend(self.check_config()?);
            }
        }

        // Update report based on findings
        let has_errors = findings
            .iter()
            .any(|f| f.severity >= DoctorSeverity::Error);

        if has_errors {
            report.status = ProxyStatus::Unknown("errors found".into());
        }

        // Log findings
        for finding in &findings {
            match finding.severity {
                DoctorSeverity::Info => tracing::info!("[{}] {}", finding.id, finding.title),
                DoctorSeverity::Warning => tracing::warn!("[{}] {}", finding.id, finding.title),
                DoctorSeverity::Error => tracing::error!("[{}] {}", finding.id, finding.title),
                DoctorSeverity::Critical => tracing::error!("[{}] {}", finding.id, finding.title),
            }
        }

        Ok(report)
    }

    /// Check proxy service status.
    fn check_service(&self) -> Result<Vec<DoctorFinding>> {
        let mut findings = Vec::new();

        // Check if nginx is running
        let spec = CommandSpec::new("systemctl").args(["status", "nginx"]);
        let output = self.runner.run(&spec)?;
        let status = parse_nginx_status(&output.stdout);

        if status.running {
            findings.push(
                DoctorFinding::new(
                    "nginx.service.running",
                    DoctorSeverity::Info,
                    "Nginx service is running",
                )
                .detail(format!("PID: {:?}", status.pid)),
            );
        } else {
            findings.push(
                DoctorFinding::new(
                    "nginx.service.not-running",
                    DoctorSeverity::Error,
                    "Nginx service is not running",
                )
                .fix("Start nginx: systemctl start nginx"),
            );
        }

        // Check nginx version
        let spec = CommandSpec::new("nginx").arg("-v");
        let version_output = self.runner.run(&spec)?;
        if let Some(version) = parse_nginx_version(&version_output.stderr) {
            findings.push(
                DoctorFinding::new(
                    "nginx.version",
                    DoctorSeverity::Info,
                    "Nginx version detected",
                )
                .detail(format!("Version: {version}")),
            );
        }

        Ok(findings)
    }

    /// Check Nginx configuration validity.
    fn check_config(&self) -> Result<Vec<DoctorFinding>> {
        let mut findings = Vec::new();

        let spec = CommandSpec::new("nginx").arg("-t");
        let output = self.runner.run(&spec)?;
        if output.success {
            findings.push(DoctorFinding::new(
                "nginx.config.valid",
                DoctorSeverity::Info,
                "Nginx configuration is valid",
            ));
        } else {
            findings.push(
                DoctorFinding::new(
                    "nginx.config.invalid",
                    DoctorSeverity::Critical,
                    "Nginx configuration has syntax errors",
                )
                .detail(output.stderr.clone())
                .fix("Fix the syntax errors and run 'nginx -t' to verify"),
            );
        }

        Ok(findings)
    }

    /// Check security headers.
    fn check_headers(&self) -> Result<Vec<DoctorFinding>> {
        let mut findings = Vec::new();

        // Check if security headers snippet exists
        let snippet_path = self.paths.nginx_snippets.join("security-headers.conf");
        if snippet_path.exists() {
            findings.push(DoctorFinding::new(
                "nginx.headers.security-headers",
                DoctorSeverity::Info,
                "Security headers snippet exists",
            ));
        } else {
            findings.push(
                DoctorFinding::new(
                    "nginx.headers.missing",
                    DoctorSeverity::Warning,
                    "Security headers snippet not found",
                )
                .detail(format!(
                    "Expected at {}",
                    snippet_path.display()
                ))
                .fix("Create a security headers snippet in nginx/snippets/"),
            );
        }

        Ok(findings)
    }

    /// Check certificate expiry.
    fn check_certificates(&self) -> Result<Vec<DoctorFinding>> {
        let mut findings = Vec::new();

        // List certificates in the certbot live directory
        if self.paths.certbot_live_dir.is_dir() {
            let entries = std::fs::read_dir(&self.paths.certbot_live_dir);
            if let Ok(entries) = entries {
                for entry in entries.flatten() {
                    let domain = entry
                        .file_name()
                        .to_string_lossy()
                        .to_string();
                    let cert_path = entry.path().join("fullchain.pem");

                    if !cert_path.exists() {
                        findings.push(
                            DoctorFinding::new(
                                "cert.missing-cert",
                                DoctorSeverity::Warning,
                                format!("Certificate file missing for {domain}"),
                            )
                            .detail(format!("Expected at {}", cert_path.display()))
                            .fix("Re-obtain the certificate with certbot"),
                        );
                    }
                }
            }
        } else {
            findings.push(DoctorFinding::new(
                "cert.no-certbot-dir",
                DoctorSeverity::Info,
                "No certbot live directory found",
            ));
        }

        Ok(findings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_finding_builder() {
        let finding = DoctorFinding::new(
            "test.finding",
            DoctorSeverity::Warning,
            "Test finding",
        )
        .detail("Some detail")
        .fix("Some fix");

        assert_eq!(finding.id, "test.finding");
        assert_eq!(finding.severity, DoctorSeverity::Warning);
        assert_eq!(finding.fix, Some("Some fix".into()));
    }
}
