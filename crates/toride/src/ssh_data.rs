//! Async SSH data collection.
//!
//! [`SshDataCollector`] manages background collection of SSH key data via a
//! tokio oneshot channel, following the same pattern as [`StatusCollector`].
//!
//! Currently seeds mock data. Will be wired to [`SshManager`] in a later phase.

use tokio::sync::oneshot;

use crate::ui::screens::ssh::SshKeyEntry;

/// Manages periodic async collection of SSH key data.
pub struct SshDataCollector {
    rx: Option<oneshot::Receiver<Vec<SshKeyEntry>>>,
}

impl SshDataCollector {
    /// Create a new collector with no pending collection.
    #[must_use]
    pub fn new() -> Self {
        Self { rx: None }
    }

    /// Whether a collection is currently in-flight.
    pub fn is_pending(&self) -> bool {
        self.rx.is_some()
    }

    /// Start a new background collection.
    ///
    /// If a collection is already in-flight, this is a no-op.
    pub fn start(&mut self) {
        if self.rx.is_some() {
            return;
        }
        let (tx, rx) = oneshot::channel();
        self.rx = Some(rx);
        tokio::spawn(async move {
            let keys = collect_mock_keys();
            let _ = tx.send(keys);
        });
    }

    /// Poll for a completed collection result.
    ///
    /// Returns `Some(keys)` if the collection completed, `None` if still
    /// pending or if the collection failed.
    pub async fn poll(&mut self) -> Option<Vec<SshKeyEntry>> {
        match &mut self.rx {
            Some(rx) => {
                let result = rx.await.ok();
                self.rx = None;
                result
            }
            None => None,
        }
    }
}

impl Default for SshDataCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Collect mock SSH key data for development.
///
/// TODO: Replace with `SshManager::keys().list()` calls in Phase 2.
fn collect_mock_keys() -> Vec<SshKeyEntry> {
    vec![
        SshKeyEntry {
            name: "id_ed25519".into(),
            key_type: "Ed25519".into(),
            fingerprint: "SHA256:abc123def456ghi789".into(),
            encrypted: true,
            permissions: "0600".into(),
            has_public: true,
            has_cert: false,
            host_count: 2,
        },
        SshKeyEntry {
            name: "id_rsa".into(),
            key_type: "RSA 4096".into(),
            fingerprint: "SHA256:xyz789abc456def123".into(),
            encrypted: false,
            permissions: "0644".into(),
            has_public: true,
            has_cert: true,
            host_count: 0,
        },
        SshKeyEntry {
            name: "deploy_key".into(),
            key_type: "Ed25519".into(),
            fingerprint: "SHA256:qwe456rty789uio012".into(),
            encrypted: false,
            permissions: "0600".into(),
            has_public: true,
            has_cert: false,
            host_count: 5,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_not_pending() {
        let collector = SshDataCollector::new();
        assert!(!collector.is_pending());
    }

    #[test]
    fn default_matches_new() {
        assert_eq!(SshDataCollector::new().is_pending(), SshDataCollector::default().is_pending());
    }

    #[tokio::test]
    async fn start_makes_pending() {
        let mut collector = SshDataCollector::new();
        assert!(!collector.is_pending());
        collector.start();
        assert!(collector.is_pending());
    }

    #[tokio::test]
    async fn start_is_idempotent() {
        let mut collector = SshDataCollector::new();
        collector.start();
        collector.start();
        assert!(collector.is_pending());
    }

    #[tokio::test]
    async fn poll_returns_keys_after_collection() {
        let mut collector = SshDataCollector::new();
        collector.start();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let result = collector.poll().await;
        assert!(result.is_some());
        let keys = result.unwrap();
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].name, "id_ed25519");
    }

    #[tokio::test]
    async fn poll_clears_pending() {
        let mut collector = SshDataCollector::new();
        collector.start();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let _ = collector.poll().await;
        assert!(!collector.is_pending());
    }

    #[tokio::test]
    async fn poll_returns_none_when_not_started() {
        let mut collector = SshDataCollector::new();
        let result = collector.poll().await;
        assert!(result.is_none());
    }

    #[test]
    fn mock_keys_have_expected_content() {
        let keys = collect_mock_keys();
        assert_eq!(keys.len(), 3);
        assert!(keys.iter().all(|k| !k.name.is_empty()));
        assert!(keys.iter().all(|k| !k.key_type.is_empty()));
        assert!(keys.iter().all(|k| !k.fingerprint.is_empty()));
    }
}
