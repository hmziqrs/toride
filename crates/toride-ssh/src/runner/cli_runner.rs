//! Trait abstraction for running external CLI tools.
//!
//! The [`CliRunner`] trait decouples the rest of the crate from the concrete
//! `duct`-based process spawning, making it straightforward to substitute a
//! mock in tests.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use async_trait::async_trait;

use crate::{Error, Result};

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Abstraction over external command execution.
///
/// Production code uses [`DefaultCliRunner`]; tests swap in [`MockCliRunner`].
#[async_trait]
pub trait CliRunner: Send + Sync {
    /// Run an external command and return its captured stdout.
    ///
    /// Implementations MUST forward the call through
    /// `tokio::task::spawn_blocking` (or equivalent) so that the async
    /// runtime is never blocked.
    async fn run(&self, cmd: &str, args: Vec<String>) -> Result<String>;

    /// Return `true` when the given tool is available on `PATH`.
    fn tool_exists(&self, name: &str) -> bool;
}

// ---------------------------------------------------------------------------
// DefaultCliRunner  (production – wraps `duct`)
// ---------------------------------------------------------------------------

/// Production [`CliRunner`] that shells out via `duct` inside
/// `tokio::task::spawn_blocking`.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultCliRunner;

#[async_trait]
impl CliRunner for DefaultCliRunner {
    async fn run(&self, cmd: &str, args: Vec<String>) -> Result<String> {
        let cmd = cmd.to_owned();
        tokio::task::spawn_blocking(move || {
            duct::cmd(&*cmd, &args)
                .read()
                .map_err(|e| Error::CommandFailed(e.to_string()))
        })
        .await
        .map_err(|e| Error::TaskFailed(e.to_string()))?
    }

    fn tool_exists(&self, name: &str) -> bool {
        which::which(name).is_ok()
    }
}

// ---------------------------------------------------------------------------
// MockCliRunner  (test double)
// ---------------------------------------------------------------------------

/// A canned-response [`CliRunner`] for unit tests.
///
/// Register per-command responses before exercising the code under test:
///
/// ```rust,ignore
/// use toride_ssh::runner::cli_runner::MockCliRunner;
///
/// let mock = MockCliRunner::new();
/// mock.push_run_response("ssh-keygen", Ok("key generated".into()));
/// mock.set_tool_exists("ssh-keygen", true);
/// ```
pub struct MockCliRunner {
    /// Ordered responses keyed by command name.
    run_responses: Mutex<HashMap<String, VecDeque<Result<String>>>>,
    /// Fixed answers for [`CliRunner::tool_exists`].
    tool_exists_responses: Mutex<HashMap<String, bool>>,
}

impl MockCliRunner {
    /// Create an empty mock (all calls will return errors by default).
    pub fn new() -> Self {
        Self {
            run_responses: Mutex::new(HashMap::new()),
            tool_exists_responses: Mutex::new(HashMap::new()),
        }
    }

    /// Enqueue a response for the given command name.
    ///
    /// Responses are returned in FIFO order.  If the queue for `cmd` is
    /// exhausted the mock returns [`Error::CommandFailed`].
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned.
    pub fn push_run_response(&self, cmd: &str, result: Result<String>) {
        self.run_responses
            .lock()
            .expect("mock lock poisoned")
            .entry(cmd.to_owned())
            .or_default()
            .push_back(result);
    }

    /// Set whether [`CliRunner::tool_exists`] should return `true` or `false`
    /// for the given tool name.
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned.
    pub fn set_tool_exists(&self, name: &str, exists: bool) {
        self.tool_exists_responses
            .lock()
            .expect("mock lock poisoned")
            .insert(name.to_owned(), exists);
    }
}

impl Default for MockCliRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CliRunner for MockCliRunner {
    async fn run(&self, cmd: &str, _args: Vec<String>) -> Result<String> {
        let mut map = self.run_responses.lock().expect("mock lock poisoned");
        let queue = map.get_mut(cmd);
        match queue.and_then(VecDeque::pop_front) {
            Some(result) => result,
            None => Err(Error::CommandFailed(format!(
                "mock: no response registered for `{cmd}`"
            ))),
        }
    }

    fn tool_exists(&self, name: &str) -> bool {
        *self
            .tool_exists_responses
            .lock()
            .expect("mock lock poisoned")
            .get(name)
            .unwrap_or(&false)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_run_returns_enqueued_responses_in_order() {
        let mock = MockCliRunner::new();
        mock.push_run_response("ssh-keygen", Ok("line 1".into()));
        mock.push_run_response("ssh-keygen", Ok("line 2".into()));

        let r1 = mock
            .run("ssh-keygen", vec!["-t".into(), "ed25519".into()])
            .await
            .unwrap();
        assert_eq!(r1, "line 1");

        let r2 = mock
            .run("ssh-keygen", vec!["-t".into(), "rsa".into()])
            .await
            .unwrap();
        assert_eq!(r2, "line 2");
    }

    #[tokio::test]
    async fn mock_run_returns_error_when_queue_exhausted() {
        let mock = MockCliRunner::new();
        mock.push_run_response("ssh-keygen", Ok("ok".into()));

        // First call succeeds.
        let _ = mock.run("ssh-keygen", vec![]).await.unwrap();

        // Second call has no more enqueued responses.
        let err = mock.run("ssh-keygen", vec![]).await.unwrap_err();
        assert!(matches!(err, Error::CommandFailed(_)));
    }

    #[tokio::test]
    async fn mock_run_unregistered_command_errors() {
        let mock = MockCliRunner::new();
        let err = mock.run("unknown", vec![]).await.unwrap_err();
        assert!(matches!(err, Error::CommandFailed(_)));
    }

    #[test]
    fn mock_tool_exists_default_false() {
        let mock = MockCliRunner::new();
        assert!(!mock.tool_exists("ssh-keygen"));
    }

    #[test]
    fn mock_tool_exists_respects_configured_value() {
        let mock = MockCliRunner::new();
        mock.set_tool_exists("ssh-keygen", true);
        assert!(mock.tool_exists("ssh-keygen"));
        assert!(!mock.tool_exists("ssh-keyscan"));
    }
}
