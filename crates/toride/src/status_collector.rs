//! Async status data collection.
//!
//! [`StatusCollector`] manages background collection of [`TorideStatus`]
//! via a tokio oneshot channel, spawning blocking work on the tokio thread pool.

use tokio::sync::oneshot;

use crate::status::TorideStatus;

/// Manages periodic async collection of system status.
pub struct StatusCollector {
    rx: Option<oneshot::Receiver<TorideStatus>>,
}

impl StatusCollector {
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
            let status = tokio::task::spawn_blocking(TorideStatus::collect)
                .await
                .unwrap_or_else(|_| TorideStatus::collect());
            let _ = tx.send(status);
        });
    }

    /// Poll for a completed collection result.
    ///
    /// Returns `Some(status)` if the collection completed, `None` if still
    /// pending or if the collection failed.
    pub async fn poll(&mut self) -> Option<TorideStatus> {
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

impl Default for StatusCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_not_pending() {
        let collector = StatusCollector::new();
        assert!(
            !collector.is_pending(),
            "new collector should not be pending"
        );
    }

    #[test]
    fn default_matches_new() {
        let new_collector = StatusCollector::new();
        let default_collector = StatusCollector::default();
        assert_eq!(new_collector.is_pending(), default_collector.is_pending());
    }

    #[tokio::test]
    async fn start_makes_pending() {
        let mut collector = StatusCollector::new();
        assert!(!collector.is_pending());
        collector.start();
        assert!(
            collector.is_pending(),
            "after start(), collector should be pending"
        );
    }

    #[tokio::test]
    async fn start_is_idempotent() {
        let mut collector = StatusCollector::new();
        collector.start();
        assert!(collector.is_pending());
        // Second start should be a no-op (doesn't replace the receiver)
        collector.start();
        assert!(collector.is_pending());
    }

    #[tokio::test]
    async fn poll_returns_status_after_collection() {
        let mut collector = StatusCollector::new();
        collector.start();
        // Give the spawned task time to complete
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let result = collector.poll().await;
        assert!(
            result.is_some(),
            "poll should return Some after collection completes"
        );
        let status = result.unwrap();
        assert!(
            !status.system.hostname.is_empty(),
            "collected status should have a hostname"
        );
    }

    #[tokio::test]
    async fn poll_clears_pending() {
        let mut collector = StatusCollector::new();
        collector.start();
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let _ = collector.poll().await;
        assert!(!collector.is_pending(), "poll should clear pending state");
    }

    #[tokio::test]
    async fn poll_returns_none_when_not_started() {
        let mut collector = StatusCollector::new();
        let result = collector.poll().await;
        assert!(
            result.is_none(),
            "poll on unstarted collector should return None"
        );
    }
}
