//! Config file read/write operations for user management.
//!
//! Provides functions to read and write the toride-users configuration
//! file, which stores managed user specifications and policies.

use std::path::Path;

use crate::spec::UserSpec;
use crate::{Error, Result};

/// Default config file name.
const CONFIG_FILENAME: &str = "users.json";

/// Toride-users configuration file containing managed user specs.
///
/// Requires the `serde` feature for JSON serialization.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UsersConfig {
    /// Managed user specifications.
    pub users: Vec<UserSpec>,
}

impl UsersConfig {
    /// Create an empty config.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Read config from a JSON file.
    ///
    /// Requires the `serde` feature.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the file cannot be read, or [`Error::Other`]
    /// if the JSON is malformed.
    pub fn read(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json(&content)
    }

    /// Parse config from a JSON string.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Other`] if the JSON is malformed.
    pub fn from_json(json: &str) -> Result<Self> {
        // When serde is not enabled, provide a stub that returns an error
        #[cfg(feature = "serde")]
        {
            serde_json::from_str(json)
                .map_err(|e| Error::Other(format!("config parse error: {e}")))
        }
        #[cfg(not(feature = "serde"))]
        {
            let _ = json;
            Err(Error::Other("serde feature is required for config parsing".into()))
        }
    }

    /// Write config to a JSON file.
    ///
    /// Creates a backup before writing. Requires the `serde` feature.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the file cannot be written.
    pub fn write(&self, path: &Path) -> Result<()> {
        if path.exists() {
            crate::backup::backup_file(path, None)?;
        }

        let content = self.to_json()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, content)?;
        Ok(())
    }

    /// Serialize config to a JSON string.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Other`] if serialization fails.
    pub fn to_json(&self) -> Result<String> {
        #[cfg(feature = "serde")]
        {
            serde_json::to_string_pretty(self)
                .map_err(|e| Error::Other(format!("config serialize error: {e}")))
        }
        #[cfg(not(feature = "serde"))]
        {
            Err(Error::Other("serde feature is required for config serialization".into()))
        }
    }

    /// Resolve the default config path using XDG conventions.
    ///
    /// Returns `~/.config/toride/users/users.json`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Other`] if the config directory cannot be determined.
    pub fn default_path() -> Result<std::path::PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| Error::Other("cannot determine config directory".into()))?;
        Ok(config_dir.join("toride").join("users").join(CONFIG_FILENAME))
    }

    /// Add or update a user spec in the config.
    pub fn upsert_user(&mut self, spec: UserSpec) {
        if let Some(existing) = self.users.iter_mut().find(|u| u.username == spec.username) {
            *existing = spec;
        } else {
            self.users.push(spec);
        }
    }

    /// Remove a user spec from the config.
    pub fn remove_user(&mut self, username: &str) -> bool {
        let original_len = self.users.len();
        self.users.retain(|u| u.username != username);
        self.users.len() != original_len
    }

    /// Get a user spec by username.
    #[must_use]
    pub fn get_user(&self, username: &str) -> Option<&UserSpec> {
        self.users.iter().find(|u| u.username == username)
    }
}
