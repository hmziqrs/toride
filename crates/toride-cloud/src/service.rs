//! Service management for cloud provider agents.
//!
//! Manages the lifecycle of cloud provider agents and helper services
//! (e.g. the AWS SSM agent, GCP guest agent, etc.).

use crate::error::Result;
use crate::CloudProvider;

// ---------------------------------------------------------------------------
// ServiceManager
// ---------------------------------------------------------------------------

/// Manages cloud provider services on the current machine.
///
/// Provides methods for checking service status, enabling/disabling services,
/// and restarting cloud provider agents.
pub struct ServiceManager {
    /// The cloud provider whose services are being managed.
    pub provider: CloudProvider,
}

impl ServiceManager {
    /// Create a new service manager for the given provider.
    #[must_use]
    pub fn new(provider: CloudProvider) -> Self {
        Self { provider }
    }

    /// Create a service manager by auto-detecting the provider.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ProviderNotFound`] if no provider can be detected.
    pub fn detect() -> Result<Self> {
        let provider = crate::detect::detect_provider()?;
        Ok(Self { provider })
    }

    /// Check if the provider's agent service is running.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the check fails.
    pub fn is_agent_running(&self) -> Result<bool> {
        // TODO: Implement provider-specific agent checks.
        Ok(false)
    }

    /// Start the provider's agent service.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the start fails.
    pub fn start_agent(&self) -> Result<()> {
        // TODO: Implement provider-specific agent start.
        Ok(())
    }

    /// Stop the provider's agent service.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the stop fails.
    pub fn stop_agent(&self) -> Result<()> {
        // TODO: Implement provider-specific agent stop.
        Ok(())
    }

    /// Restart the provider's agent service.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the restart fails.
    pub fn restart_agent(&self) -> Result<()> {
        self.stop_agent()?;
        self.start_agent()
    }

    /// Check if the provider's agent service is enabled at boot.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the check fails.
    pub fn is_agent_enabled(&self) -> Result<bool> {
        // TODO: Implement systemd/launchd enabled check.
        Ok(false)
    }

    /// Enable the provider's agent service at boot.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the enable fails.
    pub fn enable_agent(&self) -> Result<()> {
        // TODO: Implement systemd/launchd enable.
        Ok(())
    }

    /// Disable the provider's agent service at boot.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the disable fails.
    pub fn disable_agent(&self) -> Result<()> {
        // TODO: Implement systemd/launchd disable.
        Ok(())
    }

    /// Return the name of the agent service for the current provider.
    #[must_use]
    pub fn agent_service_name(&self) -> &'static str {
        match self.provider {
            CloudProvider::Aws => "amazon-ssm-agent",
            CloudProvider::Gcp => "google-guest-agent",
            CloudProvider::DigitalOcean => "do-agent",
            CloudProvider::Hetzner => "hetzner-cloud-init",
            CloudProvider::Unknown => "",
        }
    }
}
