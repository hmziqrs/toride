//! Rendering functions for proxy configuration files.
//!
//! Generates Nginx server blocks, Caddyfiles, and security header snippets
//! from typed specifications.

use crate::spec::{ProxySpec, ServerBlock};

/// Render a single Nginx server block from a [`ServerBlock`] spec.
///
/// Produces a complete `server { ... }` block including listen directive,
/// server_name, TLS settings (if configured), and proxy_pass to upstream.
///
/// # Example
///
/// ```
/// use toride_proxy::render::render_nginx_server_block;
/// use toride_proxy::spec::ServerBlock;
///
/// let block = ServerBlock::new("example.com", 80, "127.0.0.1:3000");
/// let config = render_nginx_server_block(&block);
/// assert!(config.contains("server_name example.com"));
/// assert!(config.contains("proxy_pass http://127.0.0.1:3000"));
/// ```
pub fn render_nginx_server_block(block: &ServerBlock) -> String {
    let mut lines = Vec::new();

    // Server block open
    lines.push("server {".to_string());

    // Listen directive
    if block.tls.is_some() {
        lines.push(format!("    listen {} ssl http2;", block.listen_port));
        lines.push(format!("    listen [::]:{} ssl http2;", block.listen_port));
    } else {
        lines.push(format!("    listen {};", block.listen_port));
        lines.push(format!("    listen [::]:{};", block.listen_port));
    }

    // Server name
    lines.push(format!("    server_name {};", block.server_name));

    // TLS configuration
    if let Some(tls) = &block.tls {
        lines.push(String::new());
        lines.push("    # TLS configuration".into());
        lines.push(format!("    ssl_certificate {};", tls.cert_path));
        lines.push(format!("    ssl_certificate_key {};", tls.key_path));

        if let Some(chain) = &tls.chain_path {
            lines.push(format!("    ssl_trusted_certificate {};", chain));
        }

        if tls.ocsp_stapling {
            lines.push("    ssl_stapling on;".into());
            lines.push("    ssl_stapling_verify on;".into());
        }
    }

    // Proxy pass
    lines.push(String::new());
    lines.push("    # Proxy configuration".into());
    lines.push(format!("    location / {{"));
    lines.push(format!("        proxy_pass http://{};", block.upstream));
    lines.push("        proxy_http_version 1.1;".into());
    lines.push("        proxy_set_header Upgrade $http_upgrade;".into());
    lines.push("        proxy_set_header Connection \"upgrade\";".into());
    lines.push("        proxy_set_header Host $host;".into());
    lines.push("        proxy_set_header X-Real-IP $remote_addr;".into());
    lines.push("        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;".into());
    lines.push("        proxy_set_header X-Forwarded-Proto $scheme;".into());
    lines.push("    }".into());

    // Extra directives
    for directive in &block.extra_directives {
        lines.push(format!("    {directive}"));
    }

    // Server block close
    lines.push("}".to_string());

    lines.join("\n")
}

/// Render a complete Caddyfile from a [`ProxySpec`].
///
/// Produces Caddy-format configuration for all server blocks in the spec.
///
/// # Example
///
/// ```
/// use toride_proxy::render::render_caddyfile;
/// use toride_proxy::spec::{ProxySpec, ServerBlock};
///
/// let spec = ProxySpec::builder()
///     .block(ServerBlock::new("example.com", 443, "127.0.0.1:3000"))
///     .build();
/// let caddyfile = render_caddyfile(&spec);
/// assert!(caddyfile.contains("example.com"));
/// assert!(caddyfile.contains("reverse_proxy 127.0.0.1:3000"));
/// ```
pub fn render_caddyfile(spec: &ProxySpec) -> String {
    let mut blocks = Vec::new();

    for block in &spec.server_blocks {
        let mut lines = Vec::new();

        // Site address
        lines.push(format!("{} {{", block.server_name));

        // Reverse proxy
        lines.push(format!("    reverse_proxy {}", block.upstream));

        // Extra directives
        for directive in &block.extra_directives {
            lines.push(format!("    {directive}"));
        }

        lines.push("}".to_string());
        blocks.push(lines.join("\n"));
    }

    blocks.join("\n\n")
}

/// Render common security headers as Nginx `add_header` directives.
///
/// Includes:
/// - `Strict-Transport-Security` (HSTS)
/// - `X-Content-Type-Options`
/// - `X-Frame-Options`
/// - `X-XSS-Protection`
/// - `Referrer-Policy`
/// - `Content-Security-Policy` (optional)
///
/// # Example
///
/// ```
/// use toride_proxy::render::render_security_headers;
///
/// let headers = render_security_headers(true, None);
/// assert!(headers.contains("Strict-Transport-Security"));
/// assert!(headers.contains("X-Content-Options"));
/// ```
pub fn render_security_headers(include_hsts: bool, csp_policy: Option<&str>) -> String {
    let mut lines = Vec::new();

    lines.push("    # Security headers".into());

    if include_hsts {
        lines.push(
            "    add_header Strict-Transport-Security \"max-age=31536000; includeSubDomains\" always;"
                .into(),
        );
    }

    lines.push(
        "    add_header X-Content-Type-Options \"nosniff\" always;".into(),
    );
    lines.push(
        "    add_header X-Frame-Options \"SAMEORIGIN\" always;".into(),
    );
    lines.push(
        "    add_header X-XSS-Protection \"1; mode=block\" always;".into(),
    );
    lines.push(
        "    add_header Referrer-Policy \"strict-origin-when-cross-origin\" always;".into(),
    );

    if let Some(csp) = csp_policy {
        lines.push(format!(
            "    add_header Content-Security-Policy \"{csp}\" always;"
        ));
    }

    lines.join("\n")
}

/// Render a complete Nginx configuration from a [`ProxySpec`].
///
/// Produces all server blocks with security headers included.
pub fn render_nginx_config(spec: &ProxySpec) -> String {
    let mut blocks = Vec::new();

    for block in &spec.server_blocks {
        let block_config = render_nginx_server_block(block);
        blocks.push(block_config);
    }

    blocks.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::TlsConfig;

    #[test]
    fn render_plaintext_server_block() {
        let block = ServerBlock::new("example.com", 80, "127.0.0.1:3000");
        let config = render_nginx_server_block(&block);

        assert!(config.contains("listen 80;"));
        assert!(config.contains("server_name example.com;"));
        assert!(config.contains("proxy_pass http://127.0.0.1:3000;"));
        assert!(!config.contains("ssl_certificate"));
    }

    #[test]
    fn render_tls_server_block() {
        let block = ServerBlock::new("example.com", 443, "127.0.0.1:3000")
            .with_tls(TlsConfig::new(
                "example.com",
                "/etc/letsencrypt/live/example.com/fullchain.pem",
                "/etc/letsencrypt/live/example.com/privkey.pem",
            ));
        let config = render_nginx_server_block(&block);

        assert!(config.contains("listen 443 ssl http2;"));
        assert!(config.contains("ssl_certificate /etc/letsencrypt/live/example.com/fullchain.pem;"));
        assert!(config.contains("ssl_stapling on;"));
    }

    #[test]
    fn render_caddyfile_basic() {
        let spec = ProxySpec::builder()
            .block(ServerBlock::new("example.com", 443, "127.0.0.1:3000"))
            .build();

        let caddyfile = render_caddyfile(&spec);
        assert!(caddyfile.contains("example.com {"));
        assert!(caddyfile.contains("reverse_proxy 127.0.0.1:3000"));
    }

    #[test]
    fn render_security_headers_with_hsts() {
        let headers = render_security_headers(true, None);
        assert!(headers.contains("Strict-Transport-Security"));
        assert!(headers.contains("X-Content-Type-Options"));
        assert!(headers.contains("X-Frame-Options"));
        assert!(!headers.contains("Content-Security-Policy"));
    }

    #[test]
    fn render_security_headers_with_csp() {
        let headers = render_security_headers(true, Some("default-src 'self'"));
        assert!(headers.contains("Content-Security-Policy"));
        assert!(headers.contains("default-src 'self'"));
    }
}
