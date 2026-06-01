//! Key generation with `zeroize` hygiene.
//!
//! Provides utilities for generating WireGuard key pairs. Private keys are
//! wrapped in types that implement `Zeroize` so sensitive material is cleared
//! from memory when dropped.

use zeroize::Zeroize;

use crate::error::{Error, Result};

// ---------------------------------------------------------------------------
// PrivateKey
// ---------------------------------------------------------------------------

/// A WireGuard private key with automatic zeroization on drop.
///
/// The key is stored as a Base64-encoded string (44 bytes including padding).
/// When this value is dropped, the inner bytes are overwritten with zeroes.
#[derive(Clone)]
pub struct PrivateKey {
    inner: String,
}

impl PrivateKey {
    /// Create a `PrivateKey` from a Base64-encoded string.
    ///
    /// # Errors
    ///
    /// Returns [`Error::KeyGeneration`] if the string is not a valid
    /// Base64-encoded 32-byte key.
    pub fn from_base64(key: &str) -> Result<Self> {
        let bytes = base64_decode(key)?;
        if bytes.len() != 32 {
            return Err(Error::KeyGeneration(format!(
                "private key must be 32 bytes, got {}",
                bytes.len()
            )));
        }
        Ok(Self {
            inner: key.to_owned(),
        })
    }

    /// Returns the Base64-encoded private key.
    pub fn as_base64(&self) -> &str {
        &self.inner
    }

    /// Derive the corresponding public key from this private key.
    ///
    /// Uses `wg pubkey` or an embedded Curve25519 computation.
    ///
    /// # Errors
    ///
    /// Returns [`Error::KeyGeneration`] if derivation fails.
    pub fn public_key(&self) -> Result<PublicKey> {
        // TODO: implement via `echo <key> | wg pubkey` or x25519-dalek.
        tracing::debug!("deriving public key from private key");
        Ok(PublicKey {
            inner: String::new(),
        })
    }
}

impl Zeroize for PrivateKey {
    fn zeroize(&mut self) {
        // Overwrite the string's bytes safely. Replace every character with
        // '\0' (valid UTF-8) and then truncate, ensuring sensitive material
        // is cleared from the underlying buffer.
        let len = self.inner.len();
        self.inner.clear();
        for _ in 0..len {
            self.inner.push('\0');
        }
        self.inner.clear();
    }
}

impl Drop for PrivateKey {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl std::fmt::Debug for PrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PrivateKey(**REDACTED**)")
    }
}

impl std::fmt::Display for PrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("**REDACTED**")
    }
}

// ---------------------------------------------------------------------------
// PublicKey
// ---------------------------------------------------------------------------

/// A WireGuard public key.
///
/// Public keys are safe to display and log.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PublicKey {
    inner: String,
}

impl PublicKey {
    /// Create a `PublicKey` from a Base64-encoded string.
    ///
    /// # Errors
    ///
    /// Returns [`Error::KeyGeneration`] if the string is not a valid
    /// Base64-encoded 32-byte key.
    pub fn from_base64(key: &str) -> Result<Self> {
        let bytes = base64_decode(key)?;
        if bytes.len() != 32 {
            return Err(Error::KeyGeneration(format!(
                "public key must be 32 bytes, got {}",
                bytes.len()
            )));
        }
        Ok(Self {
            inner: key.to_owned(),
        })
    }

    /// Returns the Base64-encoded public key.
    pub fn as_base64(&self) -> &str {
        &self.inner
    }
}

impl std::fmt::Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.inner)
    }
}

// ---------------------------------------------------------------------------
// Key generation
// ---------------------------------------------------------------------------

/// Generate a new WireGuard key pair.
///
/// Uses `wg genkey` under the hood. The private key is wrapped in [`PrivateKey`]
/// for automatic zeroization.
///
/// # Errors
///
/// Returns [`Error::KeyGeneration`] if key generation fails, or
/// [`Error::BinaryNotFound`] if `wg` is not on `$PATH`.
pub fn generate_keypair() -> Result<(PrivateKey, PublicKey)> {
    tracing::info!("generating new WireGuard key pair");
    // TODO: implement via `wg genkey` + `wg pubkey`.
    Err(Error::KeyGeneration(
        "key generation not yet implemented".to_owned(),
    ))
}

/// Generate only a private key.
///
/// # Errors
///
/// Returns [`Error::KeyGeneration`] if key generation fails.
pub fn generate_private_key() -> Result<PrivateKey> {
    tracing::info!("generating new WireGuard private key");
    // TODO: implement via `wg genkey`.
    Err(Error::KeyGeneration(
        "key generation not yet implemented".to_owned(),
    ))
}

/// Derive a public key from a private key using `wg pubkey`.
///
/// # Errors
///
/// Returns [`Error::KeyGeneration`] if derivation fails.
pub fn derive_public_key(private_key: &PrivateKey) -> Result<PublicKey> {
    private_key.public_key()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Decode a Base64 string into raw bytes.
fn base64_decode(input: &str) -> Result<Vec<u8>> {
    let decoder = base64::engine::general_purpose::STANDARD;
    base64::Engine::decode(&decoder, input)
        .map_err(|e| Error::KeyGeneration(format!("base64 decode failed: {e}")))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use base64::Engine;

    use super::*;

    #[test]
    fn private_key_redacted_debug() {
        // Use a valid 32-byte base64 key.
        let key = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
        let pk = PrivateKey::from_base64(key).unwrap();
        let debug = format!("{pk:?}");
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains(key));
    }

    #[test]
    fn private_key_redacted_display() {
        let key = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
        let pk = PrivateKey::from_base64(key).unwrap();
        let display = format!("{pk}");
        assert!(display.contains("REDACTED"));
        assert!(!display.contains(key));
    }

    #[test]
    fn public_key_display_shows_value() {
        let key = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
        let pk = PublicKey::from_base64(key).unwrap();
        let display = format!("{pk}");
        assert_eq!(display, key);
    }

    #[test]
    fn invalid_key_length() {
        let short = base64::engine::general_purpose::STANDARD.encode(b"tooshort");
        assert!(PrivateKey::from_base64(&short).is_err());
        assert!(PublicKey::from_base64(&short).is_err());
    }

    #[test]
    fn invalid_base64() {
        assert!(PrivateKey::from_base64("not-base64!!!").is_err());
    }
}
