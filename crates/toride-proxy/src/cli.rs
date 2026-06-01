//! CLI argument definitions.
//!
//! Provides clap-based argument parsing for the proxy management CLI.

/// Proxy CLI argument definitions.
#[derive(Debug, Clone, clap::Parser)]
#[command(name = "toride-proxy", about = "Reverse proxy management")]
pub struct ProxyCli {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: ProxyCommand,
}

/// Proxy subcommands.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum ProxyCommand {
    /// Check proxy status and configuration.
    Status,

    /// Run diagnostic checks.
    Doctor {
        /// Scope of doctor checks (all, service, headers, certificates, config).
        #[arg(default_value = "all")]
        scope: String,
    },

    /// Nginx-related operations.
    Nginx {
        /// Nginx subcommand.
        #[command(subcommand)]
        action: NginxAction,
    },

    /// Caddy-related operations.
    Caddy {
        /// Caddy subcommand.
        #[command(subcommand)]
        action: CaddyAction,
    },

    /// Certificate management.
    Certs {
        /// Certificate subcommand.
        #[command(subcommand)]
        action: CertAction,
    },
}

/// Nginx subcommands.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum NginxAction {
    /// Test Nginx configuration.
    Test,

    /// Reload Nginx configuration.
    Reload,

    /// Restart Nginx service.
    Restart,

    /// List configured sites.
    Sites,

    /// Enable a site.
    Enable {
        /// Domain to enable.
        domain: String,
    },

    /// Disable a site.
    Disable {
        /// Domain to disable.
        domain: String,
    },
}

/// Caddy subcommands.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum CaddyAction {
    /// Validate Caddyfile.
    Validate,

    /// Reload Caddy configuration.
    Reload,

    /// Format Caddyfile.
    Format,
}

/// Certificate subcommands.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum CertAction {
    /// List all certificates.
    List,

    /// Obtain a new certificate.
    Obtain {
        /// Domain name.
        domain: String,
        /// Email for registration.
        email: String,
        /// Webroot path for HTTP challenge.
        #[arg(long, default_value = "/var/www/html")]
        webroot: String,
    },

    /// Renew all due certificates.
    Renew,

    /// Check renewal status.
    Check,
}
