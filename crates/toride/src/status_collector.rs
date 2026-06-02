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
