//! Filesystem paths for proxy configuration.
//!
//! Centralizes all paths that toride-proxy manages, with a `with_root` override
//! for testing.

use std::path::{Path, PathBuf};

/// Filesystem paths used by toride-proxy.
///
/// Provides the default system paths for Nginx, Caddy, and certbot
/// configuration, plus a testing override via [`ProxyPaths::with_root`].
#[derive(Debug, Clone)]
pub struct ProxyPaths {
    /// Nginx configuration directory (e.g. `/etc/nginx`).
    pub nginx_conf_dir: PathBuf,
    /// Nginx sites-available directory.
    pub nginx_sites_available: PathBuf,
    /// Nginx sites-enabled directory.
    pub nginx_sites_enabled: PathBuf,
    /// Nginx main configuration file.
    pub nginx_conf: PathBuf,
    /// Nginx snippets directory for shared config fragments.
    pub nginx_snippets: PathBuf,
    /// Caddy configuration directory (e.g. `/etc/caddy`).
    pub caddy_conf_dir: PathBuf,
    /// Caddyfile path.
    pub caddyfile: PathBuf,
    /// Certbot configuration directory (e.g. `/etc/letsencrypt`).
    pub certbot_conf_dir: PathBuf,
    /// Certbot live certificates directory.
    pub certbot_live_dir: PathBuf,
    /// Certbot renewal directory.
    pub certbot_renewal_dir: PathBuf,
    /// Certbot archive directory.
    pub certbot_archive_dir: PathBuf,
    /// Backup directory for pre-mutation snapshots.
    pub backup_dir: PathBuf,
}

impl Default for ProxyPaths {
    fn default() -> Self {
        Self {
            nginx_conf_dir: PathBuf::from("/etc/nginx"),
            nginx_sites_available: PathBuf::from("/etc/nginx/sites-available"),
            nginx_sites_enabled: PathBuf::from("/etc/nginx/sites-enabled"),
            nginx_conf: PathBuf::from("/etc/nginx/nginx.conf"),
            nginx_snippets: PathBuf::from("/etc/nginx/snippets"),
            caddy_conf_dir: PathBuf::from("/etc/caddy"),
            caddyfile: PathBuf::from("/etc/caddy/Caddyfile"),
            certbot_conf_dir: PathBuf::from("/etc/letsencrypt"),
            certbot_live_dir: PathBuf::from("/etc/letsencrypt/live"),
            certbot_renewal_dir: PathBuf::from("/etc/letsencrypt/renewal"),
            certbot_archive_dir: PathBuf::from("/etc/letsencrypt/archive"),
            backup_dir: PathBuf::from("/var/lib/toride/proxy/backups"),
        }
    }
}

impl ProxyPaths {
    /// Create paths with a custom root (for testing).
    ///
    /// All standard paths are rebased under `root`.
    pub fn with_root(root: &Path) -> Self {
        Self {
            nginx_conf_dir: root.join("etc/nginx"),
            nginx_sites_available: root.join("etc/nginx/sites-available"),
            nginx_sites_enabled: root.join("etc/nginx/sites-enabled"),
            nginx_conf: root.join("etc/nginx/nginx.conf"),
            nginx_snippets: root.join("etc/nginx/snippets"),
            caddy_conf_dir: root.join("etc/caddy"),
            caddyfile: root.join("etc/caddy/Caddyfile"),
            certbot_conf_dir: root.join("etc/letsencrypt"),
            certbot_live_dir: root.join("etc/letsencrypt/live"),
            certbot_renewal_dir: root.join("etc/letsencrypt/renewal"),
            certbot_archive_dir: root.join("etc/letsencrypt/archive"),
            backup_dir: root.join("var/lib/toride/proxy/backups"),
        }
    }

    /// Return the path to the live certificate directory for a domain.
    pub fn cert_live_path(&self, domain: &str) -> PathBuf {
        self.certbot_live_dir.join(domain)
    }

    /// Return the path to the Nginx site config for a domain.
    pub fn nginx_site_path(&self, domain: &str) -> PathBuf {
        self.nginx_sites_available.join(domain)
    }

    /// Return the path to the Nginx sites-enabled symlink for a domain.
    pub fn nginx_enabled_path(&self, domain: &str) -> PathBuf {
        self.nginx_sites_enabled.join(domain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_paths_are_absolute() {
        let paths = ProxyPaths::default();
        assert!(paths.nginx_conf_dir.is_absolute());
        assert!(paths.caddyfile.is_absolute());
        assert!(paths.certbot_conf_dir.is_absolute());
    }

    #[test]
    fn with_root_rebases_all_paths() {
        let paths = ProxyPaths::with_root(Path::new("/tmp/test-root"));
        assert_eq!(
            paths.nginx_conf,
            PathBuf::from("/tmp/test-root/etc/nginx/nginx.conf")
        );
        assert_eq!(
            paths.caddyfile,
            PathBuf::from("/tmp/test-root/etc/caddy/Caddyfile")
        );
    }

    #[test]
    fn cert_live_path_joins_domain() {
        let paths = ProxyPaths::default();
        assert_eq!(
            paths.cert_live_path("example.com"),
            PathBuf::from("/etc/letsencrypt/live/example.com")
        );
    }
}
