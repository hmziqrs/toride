//! Intrusion detection system integration stub.
//!
//! Provides a placeholder for future IDS integration (e.g. OSSEC, Suricata,
//! Snort). This module defines the interface that IDS integrations should
//! implement and provides basic status checks.

// ---------------------------------------------------------------------------
// IdsStatus
// ---------------------------------------------------------------------------

/// Status of an IDS integration.
#[derive(Debug, Clone)]
pub struct IdsStatus {
    /// Whether the IDS is installed.
    pub installed: bool,
    /// Whether the IDS service is running.
    pub running: bool,
    /// The IDS backend name (e.g. "ossec", "suricata").
    pub backend: IdsBackend,
    /// Version string if available.
    pub version: Option<String>,
}

// ---------------------------------------------------------------------------
// IdsBackend
// ---------------------------------------------------------------------------

/// Supported IDS backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdsBackend {
    /// OSSEC / Wazuh host-based IDS.
    Ossec,
    /// Suricata network IDS.
    Suricata,
    /// Snort network IDS.
    Snort,
    /// No IDS backend detected.
    None,
}

impl std::fmt::Display for IdsBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ossec => write!(f, "ossec"),
            Self::Suricata => write!(f, "suricata"),
            Self::Snort => write!(f, "snort"),
            Self::None => write!(f, "none"),
        }
    }
}

// ---------------------------------------------------------------------------
// IdsManager
// ---------------------------------------------------------------------------

/// Manager for IDS integration.
///
/// Currently a stub that provides basic detection of installed IDS
/// backends. Full integration will be implemented in future releases.
pub struct IdsManager;

impl IdsManager {
    /// Create a new IDS manager.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Detect which IDS backend is installed.
    ///
    /// Checks for known IDS binaries on `$PATH`.
    #[must_use]
    pub fn detect_backend() -> IdsBackend {
        if which::which("ossec-control").is_ok() || which::which("wazuh-control").is_ok() {
            return IdsBackend::Ossec;
        }
        if which::which("suricata").is_ok() {
            return IdsBackend::Suricata;
        }
        if which::which("snort").is_ok() {
            return IdsBackend::Snort;
        }
        IdsBackend::None
    }

    /// Get the status of the detected IDS backend.
    ///
    /// This is a stub that checks for binary availability only.
    #[must_use]
    pub fn status() -> IdsStatus {
        let backend = Self::detect_backend();
        IdsStatus {
            installed: backend != IdsBackend::None,
            running: false,
            backend,
            version: None,
        }
    }
}

impl Default for IdsManager {
    fn default() -> Self {
        Self::new()
    }
}
