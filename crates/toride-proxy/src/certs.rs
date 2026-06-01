//! TLS certificate management.
//!
//! Provides high-level operations for managing TLS certificates via
//! certbot/Let's Encrypt and OpenSSL.

use crate::error::{Error, Result};
use crate::parse::parse_certbot_certs;
use crate::paths::ProxyPaths;
use crate::report::CertInfo;
use toride_runner::{CommandSpec, Runner};

/// Certificate management facade.
///
/// Owns a command runner and proxy paths, providing convenience methods for
/// certificate operations like listing, obtaining, and renewing certificates.
pub struct CertManager<'a> {
    runner: &'a dyn Runner,
    paths: &'a ProxyPaths,
}

impl<'a> CertManager<'a> {
    /// Create a new certificate manager.
    pub fn new(runner: &'a dyn Runner, paths: &'a ProxyPaths) -> Self {
        Self { runner, paths }
    }

    /// List all certificates managed by certbot.
    ///
    /// Runs `certbot certificates` and parses the output.
    ///
    /// # Errors
    ///
    /// Returns an error if the certbot command fails.
    pub fn list_certificates(&self) -> Result<Vec<CertInfo>> {
        let spec = CommandSpec::new("certbot").arg("certificates");
        let output = self.runner.run(&spec)?;

        if !output.success {
            return Err(Error::CommandFailed {
                program: "certbot".into(),
                code: output.exit_code,
                stderr: output.stderr,
            });
        }

        Ok(parse_certbot_certs(&output.stdout))
    }

    /// Obtain a new certificate for a domain using certbot.
    ///
    /// Uses the webroot authenticator with the specified webroot path.
    ///
    /// # Errors
    ///
    /// Returns an error if the certbot command fails.
    pub fn obtain_certificate(
        &self,
        domain: &str,
        email: &str,
        webroot: &str,
    ) -> Result<()> {
        let spec = CommandSpec::new("certbot")
            .args(["certonly", "--webroot", "-w"])
            .arg(webroot)
            .args(["-d"])
            .arg(domain)
            .args(["--email"])
            .arg(email)
            .args(["--agree-tos", "--non-interactive"]);

        let output = self.runner.run(&spec)?;

        if !output.success {
            return Err(Error::CertRenewal(format!(
                "failed to obtain certificate for {domain}: {}",
                output.stderr.trim()
            )));
        }

        tracing::info!("certs: obtained certificate for {}", domain);
        Ok(())
    }

    /// Renew all certificates that are due for renewal.
    ///
    /// Runs `certbot renew`.
    ///
    /// # Errors
    ///
    /// Returns an error if the renewal command fails.
    pub fn renew_all(&self) -> Result<()> {
        let spec = CommandSpec::new("certbot")
            .args(["renew", "--non-interactive"]);

        let output = self.runner.run(&spec)?;

        if !output.success {
            return Err(Error::CertRenewal(format!(
                "certificate renewal failed: {}",
                output.stderr.trim()
            )));
        }

        tracing::info!("certs: renewed certificates");
        Ok(())
    }

    /// Revoke a certificate for a domain.
    ///
    /// # Errors
    ///
    /// Returns an error if the revocation command fails.
    pub fn revoke_certificate(&self, domain: &str) -> Result<()> {
        let cert_path = self.paths.cert_live_path(domain).join("cert.pem");
        let spec = CommandSpec::new("certbot")
            .args(["revoke", "--cert-path"])
            .arg(cert_path.to_str().unwrap_or_default())
            .arg("--non-interactive");

        let output = self.runner.run(&spec)?;

        if !output.success {
            return Err(Error::CertRenewal(format!(
                "failed to revoke certificate for {domain}: {}",
                output.stderr.trim()
            )));
        }

        tracing::info!("certs: revoked certificate for {}", domain);
        Ok(())
    }

    /// Check if a certificate exists for a domain.
    pub fn certificate_exists(&self, domain: &str) -> bool {
        self.paths.cert_live_path(domain).join("fullchain.pem").exists()
    }

    /// Get the path to the full certificate chain for a domain.
    pub fn fullchain_path(&self, domain: &str) -> std::path::PathBuf {
        self.paths.cert_live_path(domain).join("fullchain.pem")
    }

    /// Get the path to the private key for a domain.
    pub fn privkey_path(&self, domain: &str) -> std::path::PathBuf {
        self.paths.cert_live_path(domain).join("privkey.pem")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certificate_exists_checks_path() {
        let dir = assert_fs::TempDir::new().unwrap();
        let paths = ProxyPaths::with_root(dir.path());

        let fake = toride_runner::fake::FakeRunner::new();
        let mgr = CertManager::new(&fake, &paths);

        assert!(!mgr.certificate_exists("example.com"));

        // Create the cert file
        let cert_dir = paths.cert_live_path("example.com");
        std::fs::create_dir_all(&cert_dir).unwrap();
        std::fs::write(cert_dir.join("fullchain.pem"), "fake cert").unwrap();

        assert!(mgr.certificate_exists("example.com"));
    }
}
