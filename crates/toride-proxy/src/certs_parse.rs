//! Certificate parsing utilities.
//!
//! Provides functions for parsing certificate files, extracting metadata,
//! and reading certificate details from the filesystem.

use crate::error::{Error, Result};
use crate::report::CertInfo;
use std::path::Path;

/// Parsed certificate file metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCert {
    /// Domain (CN) the certificate is issued for.
    pub domain: String,
    /// Certificate file path on disk.
    pub path: String,
    /// Whether the certificate file exists and is readable.
    pub exists: bool,
}

impl ParsedCert {
    /// Create a new parsed cert entry.
    pub fn new(domain: impl Into<String>, path: impl Into<String>, exists: bool) -> Self {
        Self {
            domain: domain.into(),
            path: path.into(),
            exists,
        }
    }
}

/// List all live certificates in the certbot directory.
///
/// Scans the `live` directory for subdirectories, each representing a domain
/// certificate. Returns a list of [`ParsedCert`] entries.
///
/// # Errors
///
/// Returns an error if the live directory cannot be read.
pub fn list_live_certs(live_dir: &Path) -> Result<Vec<ParsedCert>> {
    if !live_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut certs = Vec::new();

    let entries = std::fs::read_dir(live_dir).map_err(|e| {
        Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("cannot read certbot live directory: {e}"),
        ))
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let domain = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let fullchain = path.join("fullchain.pem");
        let exists = fullchain.exists();

        certs.push(ParsedCert::new(
            domain,
            fullchain.to_string_lossy().to_string(),
            exists,
        ));
    }

    // Sort by domain for deterministic output
    certs.sort_by(|a, b| a.domain.cmp(&b.domain));

    Ok(certs)
}

/// Read the PEM-encoded certificate from a fullchain file.
///
/// Extracts just the first (leaf) certificate from a full chain PEM file.
///
/// # Errors
///
/// Returns an error if the file cannot be read.
pub fn read_leaf_certificate(path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(path)?;

    // Extract the first certificate block
    let start = "-----BEGIN CERTIFICATE-----";
    let end = "-----END CERTIFICATE-----";

    let start_idx = content
        .find(start)
        .ok_or_else(|| Error::ConfigParse("no certificate found in PEM file".into()))?;

    let rest = &content[start_idx..];
    let end_idx = rest
        .find(end)
        .ok_or_else(|| Error::ConfigParse("incomplete certificate PEM block".into()))?;

    Ok(rest[..end_idx + end.len()].to_string())
}

/// Check if a certificate file appears to be a valid PEM certificate.
///
/// Performs a quick check for PEM header/footer markers.
pub fn is_pem_certificate(path: &Path) -> bool {
    if let Ok(content) = std::fs::read_to_string(path) {
        content.contains("-----BEGIN CERTIFICATE-----")
            && content.contains("-----END CERTIFICATE-----")
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_live_certs_empty_dir() {
        let dir = assert_fs::TempDir::new().unwrap();
        let certs = list_live_certs(dir.path()).unwrap();
        assert!(certs.is_empty());
    }

    #[test]
    fn list_live_certs_finds_domains() {
        let dir = assert_fs::TempDir::new().unwrap();
        let live_dir = dir.path().join("live");
        std::fs::create_dir_all(live_dir.join("example.com")).unwrap();
        std::fs::write(
            live_dir.join("example.com/fullchain.pem"),
            "fake cert",
        )
        .unwrap();
        std::fs::create_dir_all(live_dir.join("other.com")).unwrap();
        // No cert file for other.com

        let certs = list_live_certs(&live_dir).unwrap();
        assert_eq!(certs.len(), 2);
        assert_eq!(certs[0].domain, "example.com");
        assert!(certs[0].exists);
        assert_eq!(certs[1].domain, "other.com");
        assert!(!certs[1].exists);
    }

    #[test]
    fn read_leaf_certificate_extracts_first() {
        let dir = assert_fs::TempDir::new().unwrap();
        let path = dir.path().join("fullchain.pem");
        std::fs::write(
            &path,
            "-----BEGIN CERTIFICATE-----\nleafdata\n-----END CERTIFICATE-----\n\
             -----BEGIN CERTIFICATE-----\nchaindata\n-----END CERTIFICATE-----\n",
        )
        .unwrap();

        let leaf = read_leaf_certificate(&path).unwrap();
        assert!(leaf.contains("leafdata"));
        assert!(!leaf.contains("chaindata"));
    }

    #[test]
    fn is_pem_certificate_checks_markers() {
        let dir = assert_fs::TempDir::new().unwrap();
        let path = dir.path().join("cert.pem");
        std::fs::write(
            &path,
            "-----BEGIN CERTIFICATE-----\ndata\n-----END CERTIFICATE-----\n",
        )
        .unwrap();
        assert!(is_pem_certificate(&path));

        let bad_path = dir.path().join("bad.pem");
        std::fs::write(&bad_path, "not a cert").unwrap();
        assert!(!is_pem_certificate(&bad_path));
    }
}
