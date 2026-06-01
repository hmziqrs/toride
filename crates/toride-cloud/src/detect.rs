//! Cloud provider detection via metadata endpoints.
//!
//! Determines which cloud provider the current machine is running on by
//! querying provider-specific metadata endpoints. Each cloud provider exposes
//! a unique metadata service that can be used for identification.

use std::fmt;

use crate::Result;

// ---------------------------------------------------------------------------
// CloudProvider
// ---------------------------------------------------------------------------

/// Supported cloud providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CloudProvider {
    /// Amazon Web Services (EC2).
    Aws,
    /// Google Cloud Platform (GCE).
    Gcp,
    /// DigitalOcean (Droplets).
    DigitalOcean,
    /// Hetzner Cloud.
    Hetzner,
    /// Unknown or on-premises provider.
    Unknown,
}

impl fmt::Display for CloudProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Aws => write!(f, "aws"),
            Self::Gcp => write!(f, "gcp"),
            Self::DigitalOcean => write!(f, "digitalocean"),
            Self::Hetzner => write!(f, "hetzner"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl CloudProvider {
    /// Returns all known (non-unknown) providers.
    #[must_use]
    pub const fn all() -> &'static [CloudProvider] {
        &[Self::Aws, Self::Gcp, Self::DigitalOcean, Self::Hetzner]
    }

    /// Returns the metadata endpoint URL for this provider.
    #[must_use]
    pub fn metadata_url(&self) -> Option<&'static str> {
        match self {
            Self::Aws => Some("http://169.254.169.254/latest/meta-data/"),
            Self::Gcp => Some("http://metadata.google.internal/computeMetadata/v1/"),
            Self::DigitalOcean => Some("http://169.254.169.254/metadata/v1/"),
            Self::Hetzner => Some("http://169.254.169.254/hetzner/v1/metadata/"),
            Self::Unknown => None,
        }
    }

    /// Returns the CLI tool name for this provider.
    #[must_use]
    pub fn cli_tool(&self) -> &'static str {
        match self {
            Self::Aws => "aws",
            Self::Gcp => "gcloud",
            Self::DigitalOcean => "doctl",
            Self::Hetzner => "hcloud",
            Self::Unknown => "",
        }
    }

    /// Parse a provider from a string identifier (case-insensitive).
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "aws" | "ec2" => Self::Aws,
            "gcp" | "gce" | "google" => Self::Gcp,
            "do" | "digitalocean" => Self::DigitalOcean,
            "hetzner" | "hcloud" => Self::Hetzner,
            _ => Self::Unknown,
        }
    }
}

// ---------------------------------------------------------------------------
// Provider detection
// ---------------------------------------------------------------------------

/// Detect the current cloud provider by querying metadata endpoints.
///
/// Tries each provider's metadata endpoint in order. The first one that
/// responds successfully identifies the provider.
///
/// # Errors
///
/// Returns [`Error::ProviderNotFound`] if no metadata endpoint responds.
pub fn detect_provider() -> Result<CloudProvider> {
    // TODO: Implement actual metadata endpoint probing.
    // For now, try to detect from environment variables or files.
    if let Some(provider) = detect_from_env() {
        return Ok(provider);
    }

    if let Some(provider) = detect_from_files() {
        return Ok(provider);
    }

    Ok(CloudProvider::Unknown)
}

/// Try to detect the cloud provider from well-known environment variables.
fn detect_from_env() -> Option<CloudProvider> {
    // AWS
    if std::env::var("AWS_EXECUTION_ENV").is_ok()
        || std::env::var("AWS_REGION").is_ok()
        || std::env::var("AWS_DEFAULT_REGION").is_ok()
    {
        return Some(CloudProvider::Aws);
    }

    // GCP
    if std::env::var("GCE_METADATA_HOST").is_ok()
        || std::env::var("GOOGLE_CLOUD_PROJECT").is_ok()
    {
        return Some(CloudProvider::Gcp);
    }

    // DigitalOcean
    if std::env::var("DIGITALOCEAN_TOKEN").is_ok() {
        return Some(CloudProvider::DigitalOcean);
    }

    // Hetzner
    if std::env::var("HCLOUD_TOKEN").is_ok() {
        return Some(CloudProvider::Hetzner);
    }

    None
}

/// Try to detect the cloud provider from well-known system files.
fn detect_from_files() -> Option<CloudProvider> {
    // AWS
    if std::path::Path::new("/sys/class/dmi/id/board_vendor")
        .exists()
    {
        if let Ok(vendor) = std::fs::read_to_string("/sys/class/dmi/id/board_vendor") {
            if vendor.trim() == "Amazon EC2" {
                return Some(CloudProvider::Aws);
            }
        }
    }

    // GCP
    if std::path::Path::new("/sys/class/dmi/id/product_name").exists() {
        if let Ok(name) = std::fs::read_to_string("/sys/class/dmi/id/product_name") {
            if name.trim().starts_with("Google Compute Engine") {
                return Some(CloudProvider::Gcp);
            }
        }
    }

    // DigitalOcean
    if std::path::Path::new("/sys/class/dmi/id/sys_vendor").exists() {
        if let Ok(vendor) = std::fs::read_to_string("/sys/class/dmi/id/sys_vendor") {
            if vendor.trim() == "DigitalOcean" {
                return Some(CloudProvider::DigitalOcean);
            }
        }
    }

    // Hetzner
    if std::path::Path::new("/sys/class/dmi/id/board_vendor").exists() {
        if let Ok(vendor) = std::fs::read_to_string("/sys/class/dmi/id/board_vendor") {
            let v = vendor.trim();
            if v == "Hetzner" || v == "hcloud" {
                return Some(CloudProvider::Hetzner);
            }
        }
    }

    None
}
