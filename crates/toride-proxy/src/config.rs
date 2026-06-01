//! Proxy configuration file management.
//!
//! Provides reading, writing, and validation of proxy configuration files
//! with support for both Nginx and Caddy formats.

use crate::error::{Error, Result};
use crate::nginx_config::{parse_server_blocks, ParsedServerBlock};
use crate::paths::ProxyPaths;
use crate::spec::{ProxySpec, ServerBlock, TlsConfig};

/// Configuration file manager for proxy settings.
///
/// Handles reading existing configurations from disk and writing new ones,
/// with automatic backup creation before overwriting.
pub struct ConfigManager<'a> {
    paths: &'a ProxyPaths,
}

impl<'a> ConfigManager<'a> {
    /// Create a new config manager.
    pub fn new(paths: &'a ProxyPaths) -> Self {
        Self { paths }
    }

    /// Read the main Nginx configuration file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn read_nginx_config(&self) -> Result<String> {
        std::fs::read_to_string(&self.paths.nginx_conf).map_err(|e| {
            Error::NotFound(format!(
                "cannot read nginx.conf: {} ({})",
                self.paths.nginx_conf.display(),
                e
            ))
        })
    }

    /// Parse all server blocks from the main Nginx configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be parsed.
    pub fn parse_nginx_server_blocks(&self) -> Result<Vec<ParsedServerBlock>> {
        let content = self.read_nginx_config()?;
        parse_server_blocks(&content)
    }

    /// Read a specific site configuration file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn read_site_config(&self, domain: &str) -> Result<String> {
        let path = self.paths.nginx_site_path(domain);
        std::fs::read_to_string(&path).map_err(|e| {
            Error::NotFound(format!(
                "cannot read site config for {domain}: {} ({})",
                path.display(),
                e
            ))
        })
    }

    /// List all configured site domains in sites-available.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be read.
    pub fn list_sites(&self) -> Result<Vec<String>> {
        let mut sites = Vec::new();

        if !self.paths.nginx_sites_available.is_dir() {
            return Ok(sites);
        }

        let entries = std::fs::read_dir(&self.paths.nginx_sites_available)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                sites.push(name);
            }
        }

        sites.sort();
        Ok(sites)
    }

    /// List all enabled sites (symlinks in sites-enabled).
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be read.
    pub fn list_enabled_sites(&self) -> Result<Vec<String>> {
        let mut sites = Vec::new();

        if !self.paths.nginx_sites_enabled.is_dir() {
            return Ok(sites);
        }

        let entries = std::fs::read_dir(&self.paths.nginx_sites_enabled)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() || path.is_symlink() {
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                sites.push(name);
            }
        }

        sites.sort();
        Ok(sites)
    }

    /// Read the Caddyfile.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn read_caddyfile(&self) -> Result<String> {
        std::fs::read_to_string(&self.paths.caddyfile).map_err(|e| {
            Error::NotFound(format!(
                "cannot read Caddyfile: {} ({})",
                self.paths.caddyfile.display(),
                e
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_sites_returns_empty_for_missing_dir() {
        let dir = assert_fs::TempDir::new().unwrap();
        let paths = ProxyPaths::with_root(dir.path());
        let mgr = ConfigManager::new(&paths);
        assert!(mgr.list_sites().unwrap().is_empty());
    }

    #[test]
    fn list_sites_finds_configs() {
        let dir = assert_fs::TempDir::new().unwrap();
        let paths = ProxyPaths::with_root(dir.path());
        let sites_dir = &paths.nginx_sites_available;
        std::fs::create_dir_all(sites_dir).unwrap();
        std::fs::write(sites_dir.join("b.com"), "config b").unwrap();
        std::fs::write(sites_dir.join("a.com"), "config a").unwrap();

        let mgr = ConfigManager::new(&paths);
        let sites = mgr.list_sites().unwrap();
        assert_eq!(sites, vec!["a.com", "b.com"]);
    }
}
