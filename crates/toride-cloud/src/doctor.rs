//! Diagnostic engine for cloud provider installations.
//!
//! [`Doctor`] runs structured diagnostic checks across a cloud provider
//! installation and returns a [`CloudReport`] containing typed
//! [`Finding`](crate::report::Finding) values with severity levels,
//! human-readable descriptions, and suggested fixes.
//!
//! # Categories
//!
//! Each category corresponds to a [`DoctorScope`] variant and a `check_*`
//! method on [`Doctor`]:
//!
//! | Scope             | What it checks                                    |
//! |-------------------|---------------------------------------------------|
//! | `Provider`        | Cloud provider detection and metadata             |
//! | `Binaries`        | CLI tools (aws, gcloud, doctl, hcloud)           |
//! | `SecurityGroups`  | Firewall rules, open ports, overly permissive     |
//! | `Agent`           | Provider agent running and enabled                |
//! | `Network`         | VPC/network configuration and connectivity        |
//! | `All`             | All of the above                                  |

use crate::error::Result;
use crate::report::{CloudReport, Finding, Severity};
use crate::CloudProvider;

// ---------------------------------------------------------------------------
// DoctorScope
// ---------------------------------------------------------------------------

/// Scope for diagnostic checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoctorScope {
    /// Check cloud provider detection and metadata.
    Provider,
    /// Check CLI tool availability and versions.
    Binaries,
    /// Check security groups and firewall rules.
    SecurityGroups,
    /// Check provider agent service status.
    Agent,
    /// Check network configuration.
    Network,
    /// Run all checks.
    All,
}

// ---------------------------------------------------------------------------
// Doctor
// ---------------------------------------------------------------------------

/// Diagnostic engine for cloud provider installations.
///
/// # Example
///
/// ```ignore
/// use toride_cloud::doctor::{Doctor, DoctorScope};
///
/// let doctor = Doctor::new();
/// let report = doctor.run(&DoctorScope::All)?;
///
/// if report.has_errors() {
///     for f in &report.findings {
///         eprintln!("[{}] {}", f.severity, f.title);
///     }
/// }
/// ```
pub struct Doctor {
    /// The cloud provider to diagnose.
    provider: CloudProvider,
}

impl Doctor {
    /// Create a new doctor for the auto-detected provider.
    pub fn detect() -> Result<Self> {
        let provider = crate::detect::detect_provider()?;
        Ok(Self { provider })
    }

    /// Create a new doctor for a specific provider.
    #[must_use]
    pub fn new(provider: CloudProvider) -> Self {
        Self { provider }
    }

    /// Run diagnostic checks for the given scope.
    ///
    /// # Errors
    ///
    /// Returns an error only for fundamental failures. Individual check
    /// failures appear as [`Finding`] values in the report.
    pub fn run(&self, scope: &DoctorScope) -> Result<CloudReport> {
        let mut report = CloudReport::new(self.provider);

        match scope {
            DoctorScope::Provider => self.check_provider(&mut report),
            DoctorScope::Binaries => self.check_binaries(&mut report),
            DoctorScope::SecurityGroups => self.check_security_groups(&mut report),
            DoctorScope::Agent => self.check_agent(&mut report),
            DoctorScope::Network => self.check_network(&mut report),
            DoctorScope::All => {
                self.check_provider(&mut report);
                self.check_binaries(&mut report);
                self.check_security_groups(&mut report);
                self.check_agent(&mut report);
                self.check_network(&mut report);
            }
        }

        Ok(report)
    }

    // -----------------------------------------------------------------------
    // Check methods
    // -----------------------------------------------------------------------

    /// Check cloud provider detection and metadata.
    fn check_provider(&self, report: &mut CloudReport) {
        if matches!(self.provider, CloudProvider::Unknown) {
            report.push(
                Finding::new(
                    "provider.unknown",
                    Severity::Warning,
                    "Cloud provider could not be detected",
                )
                .detail("No cloud provider metadata endpoint responded.")
                .fix("Verify the machine is running on a supported cloud provider."),
            );
        }
    }

    /// Check CLI tool availability.
    fn check_binaries(&self, report: &mut CloudReport) {
        let tool = self.provider.cli_tool();
        if tool.is_empty() {
            return;
        }

        match which::which(tool) {
            Ok(_) => {
                report.push(
                    Finding::new(
                        format!("binaries.{tool}.found"),
                        Severity::Ok,
                        format!("{tool} CLI is installed"),
                    ),
                );
            }
            Err(_) => {
                report.push(
                    Finding::new(
                        format!("binaries.{tool}.missing"),
                        Severity::Warning,
                        format!("{tool} CLI is not installed"),
                    )
                    .detail(format!("The {tool} command was not found on $PATH."))
                    .fix(format!("Install the {tool} CLI tool.")),
                );
            }
        }
    }

    /// Check security groups and firewall rules.
    fn check_security_groups(&self, _report: &mut CloudReport) {
        // TODO: Implement security group checks:
        // - Open ingress from 0.0.0.0/0 on sensitive ports
        // - Missing egress rules
        // - Unused security groups
    }

    /// Check provider agent service status.
    fn check_agent(&self, _report: &mut CloudReport) {
        // TODO: Implement agent checks:
        // - Agent service is running
        // - Agent service is enabled at boot
    }

    /// Check network configuration.
    fn check_network(&self, _report: &mut CloudReport) {
        // TODO: Implement network checks:
        // - VPC configuration
        // - Public IP exposure
        // - DNS configuration
    }
}
