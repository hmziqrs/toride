//! Nginx reverse proxy management.
//!
//! Provides high-level operations for managing Nginx configuration including
//! testing, reloading, and restarting the Nginx service.

use crate::error::{Error, Result};
use crate::paths::ProxyPaths;
use crate::render::render_nginx_server_block;
use crate::spec::ServerBlock;
use toride_runner::{CommandSpec, Runner};

/// Nginx management facade.
///
/// Owns a command runner and proxy paths, providing convenience methods for
/// Nginx operations like config testing, reloading, and site management.
pub struct NginxManager<'a> {
    runner: &'a dyn Runner,
    paths: &'a ProxyPaths,
}

impl<'a> NginxManager<'a> {
    /// Create a new Nginx manager.
    pub fn new(runner: &'a dyn Runner, paths: &'a ProxyPaths) -> Self {
        Self { runner, paths }
    }

    /// Test the current Nginx configuration (`nginx -t`).
    ///
    /// # Errors
    ///
    /// Returns [`Error::NginxSyntax`] if the configuration test fails.
    pub fn test_config(&self) -> Result<()> {
        let spec = CommandSpec::new("nginx").arg("-t");
        let output = self.runner.run(&spec)?;
        if !output.success {
            return Err(Error::NginxSyntax(output.stderr));
        }
        Ok(())
    }

    /// Reload Nginx configuration (`nginx -s reload`).
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the reload fails.
    pub fn reload(&self) -> Result<()> {
        let spec = CommandSpec::new("nginx").args(["-s", "reload"]);
        let output = self.runner.run(&spec)?;
        if !output.success {
            return Err(Error::CommandFailed {
                program: "nginx".into(),
                code: output.exit_code,
                stderr: output.stderr,
            });
        }
        Ok(())
    }

    /// Restart the Nginx service via systemctl.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the restart fails.
    pub fn restart(&self) -> Result<()> {
        let spec = CommandSpec::new("systemctl").args(["restart", "nginx"]);
        let output = self.runner.run(&spec)?;
        if !output.success {
            return Err(Error::CommandFailed {
                program: "systemctl".into(),
                code: output.exit_code,
                stderr: output.stderr,
            });
        }
        Ok(())
    }

    /// Write a server block configuration for a domain.
    ///
    /// Renders the server block to Nginx config and writes it to
    /// `sites-available`. Optionally creates a symlink in `sites-enabled`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn write_site(&self, block: &ServerBlock, enable: bool) -> Result<()> {
        block.validate()?;

        let config = render_nginx_server_block(block);
        let site_path = self.paths.nginx_site_path(&block.server_name);

        // Ensure the directory exists
        if let Some(parent) = site_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        toride_fs::atomic_write(&site_path, &config)?;

        if enable {
            self.enable_site(&block.server_name)?;
        }

        tracing::info!(
            "nginx: wrote site config for {} to {}",
            block.server_name,
            site_path.display()
        );

        Ok(())
    }

    /// Enable a site by creating a symlink in sites-enabled.
    ///
    /// # Errors
    ///
    /// Returns an error if the symlink cannot be created.
    pub fn enable_site(&self, domain: &str) -> Result<()> {
        let source = self.paths.nginx_site_path(domain);
        let link = self.paths.nginx_enabled_path(domain);

        if !source.exists() {
            return Err(Error::NotFound(format!(
                "site config not found: {}",
                source.display()
            )));
        }

        // Remove existing symlink if present
        if link.exists() {
            std::fs::remove_file(&link)?;
        }

        // Ensure sites-enabled directory exists
        if let Some(parent) = link.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::os::unix::fs::symlink(&source, &link)?;

        tracing::info!("nginx: enabled site {}", domain);
        Ok(())
    }

    /// Disable a site by removing the symlink from sites-enabled.
    ///
    /// # Errors
    ///
    /// Returns an error if the symlink cannot be removed.
    pub fn disable_site(&self, domain: &str) -> Result<()> {
        let link = self.paths.nginx_enabled_path(domain);

        if link.exists() {
            std::fs::remove_file(&link)?;
            tracing::info!("nginx: disabled site {}", domain);
        }

        Ok(())
    }

    /// Remove a site configuration file and its symlink.
    ///
    /// # Errors
    ///
    /// Returns an error if the files cannot be removed.
    pub fn remove_site(&self, domain: &str) -> Result<()> {
        self.disable_site(domain)?;

        let site_path = self.paths.nginx_site_path(domain);
        if site_path.exists() {
            std::fs::remove_file(&site_path)?;
            tracing::info!("nginx: removed site config for {}", domain);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_returns_error_on_failure() {
        let fake = toride_runner::fake::FakeRunner::new()
            .push_response(toride_runner::CommandOutput::from_stderr("syntax error", 1));

        let paths = ProxyPaths::default();
        let mgr = NginxManager::new(&fake, &paths);
        let result = mgr.test_config();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_succeeds() {
        let fake = toride_runner::fake::FakeRunner::new()
            .push_response(toride_runner::CommandOutput::from_stdout("ok"));

        let paths = ProxyPaths::default();
        let mgr = NginxManager::new(&fake, &paths);
        assert!(mgr.test_config().is_ok());
    }
}
