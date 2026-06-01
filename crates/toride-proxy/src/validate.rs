//! Validation functions for proxy configuration values.
//!
//! Ensures server names, ports, and certificate paths are well-formed
//! before they are used in configuration generation.

use crate::error::{Error, Result};

/// Validate that a server name is a valid domain name or IP address.
///
/// Server names must:
/// - Be non-empty
/// - Contain only alphanumeric characters, dots, hyphens, and asterisks (wildcards)
/// - Not start or end with a dot or hyphen
/// - Not contain consecutive dots
pub fn validate_server_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::Validation("server name must not be empty".into()));
    }

    if name.len() > 253 {
        return Err(Error::Validation(format!(
            "server name too long: {name} (max 253 characters)"
        )));
    }

    // Allow wildcard prefixes like *.example.com
    let check_name = name.strip_prefix("*.").unwrap_or(name);

    if check_name.starts_with('.') || check_name.starts_with('-') {
        return Err(Error::Validation(format!(
            "server name must not start with '.' or '-': {name}"
        )));
    }

    if check_name.ends_with('.') || check_name.ends_with('-') {
        return Err(Error::Validation(format!(
            "server name must not end with '.' or '-': {name}"
        )));
    }

    // Check for consecutive dots
    if check_name.contains("..") {
        return Err(Error::Validation(format!(
            "server name must not contain consecutive dots: {name}"
        )));
    }

    // Validate each label
    for label in check_name.split('.') {
        if label.is_empty() {
            return Err(Error::Validation(format!(
                "server name has empty label: {name}"
            )));
        }
        for ch in label.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_' {
                return Err(Error::Validation(format!(
                    "server name contains invalid character '{ch}' in: {name}"
                )));
            }
        }
    }

    Ok(())
}

/// Validate that a port number is in the valid range for TCP/UDP.
///
/// Valid ports are 1-65535. Port 0 is not valid for listening.
pub fn validate_port(port: u16) -> Result<()> {
    if port == 0 {
        return Err(Error::Validation("port must not be 0".into()));
    }
    // u16 already bounds to 65535, so we only need to check for 0
    Ok(())
}

/// Validate that a certificate path points to a plausible certificate file.
///
/// Certificate paths must:
/// - Be non-empty
/// - End with a recognized extension (.pem, .crt, .cert, .cer)
/// - Not contain path traversal sequences
pub fn validate_cert_path(path: &str) -> Result<()> {
    if path.is_empty() {
        return Err(Error::Validation("certificate path must not be empty".into()));
    }

    if path.contains("..") {
        return Err(Error::Validation(format!(
            "certificate path must not contain '..': {path}"
        )));
    }

    let valid_extensions = [".pem", ".crt", ".cert", ".cer"];
    let has_valid_ext = valid_extensions
        .iter()
        .any(|ext| path.to_ascii_lowercase().ends_with(ext));

    if !has_valid_ext {
        return Err(Error::Validation(format!(
            "certificate path must end with a recognized extension ({:?}): {path}",
            valid_extensions
        )));
    }

    Ok(())
}

/// Validate that an upstream address is well-formed.
///
/// Upstream addresses should be in the format `host:port` or a Unix socket path.
pub fn validate_upstream(upstream: &str) -> Result<()> {
    if upstream.is_empty() {
        return Err(Error::Validation("upstream address must not be empty".into()));
    }

    // Unix socket paths start with "unix:"
    if let Some(socket_path) = upstream.strip_prefix("unix:") {
        if socket_path.is_empty() {
            return Err(Error::Validation(
                "unix socket path must not be empty".into(),
            ));
        }
        return Ok(());
    }

    // host:port format
    if let Some(port_str) = upstream.rsplit_once(':').map(|(_, p)| p) {
        if port_str.parse::<u16>().is_err() {
            return Err(Error::Validation(format!(
                "upstream port must be a valid number: {upstream}"
            )));
        }
    } else {
        return Err(Error::Validation(format!(
            "upstream address must be in host:port format: {upstream}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_server_names() {
        assert!(validate_server_name("example.com").is_ok());
        assert!(validate_server_name("sub.example.com").is_ok());
        assert!(validate_server_name("*.example.com").is_ok());
        assert!(validate_server_name("my-site.example.org").is_ok());
    }

    #[test]
    fn invalid_server_names() {
        assert!(validate_server_name("").is_err());
        assert!(validate_server_name(".example.com").is_err());
        assert!(validate_server_name("example.com.").is_err());
        assert!(validate_server_name("-example.com").is_err());
        assert!(validate_server_name("example..com").is_err());
    }

    #[test]
    fn valid_ports() {
        assert!(validate_port(1).is_ok());
        assert!(validate_port(80).is_ok());
        assert!(validate_port(443).is_ok());
        assert!(validate_port(8080).is_ok());
        assert!(validate_port(65535).is_ok());
    }

    #[test]
    fn invalid_port_zero() {
        assert!(validate_port(0).is_err());
    }

    #[test]
    fn valid_cert_paths() {
        assert!(validate_cert_path("/etc/ssl/cert.pem").is_ok());
        assert!(validate_cert_path("/etc/ssl/cert.crt").is_ok());
        assert!(validate_cert_path("/etc/ssl/cert.cert").is_ok());
        assert!(validate_cert_path("/etc/ssl/cert.CER").is_ok());
    }

    #[test]
    fn invalid_cert_paths() {
        assert!(validate_cert_path("").is_err());
        assert!(validate_cert_path("/etc/ssl/../etc/cert.pem").is_err());
        assert!(validate_cert_path("/etc/ssl/cert.txt").is_err());
    }

    #[test]
    fn valid_upstreams() {
        assert!(validate_upstream("127.0.0.1:3000").is_ok());
        assert!(validate_upstream("localhost:8080").is_ok());
        assert!(validate_upstream("unix:/var/run/app.sock").is_ok());
    }

    #[test]
    fn invalid_upstreams() {
        assert!(validate_upstream("").is_err());
        assert!(validate_upstream("127.0.0.1").is_err());
        assert!(validate_upstream("host:notaport").is_err());
        assert!(validate_upstream("unix:").is_err());
    }
}
