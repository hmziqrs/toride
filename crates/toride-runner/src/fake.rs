//! Fake [`Runner`] implementation for testing.
//!
//! [`FakeRunner`] returns pre-configured responses in FIFO order and
//! records all calls for later inspection.

use std::sync::{Arc, Mutex};

use crate::output::CommandOutput;
use crate::runner::Runner;
use crate::spec::CommandSpec;
use crate::error::Result;

/// A [`Runner`] that returns canned responses for testing.
///
/// Responses are consumed in FIFO order. If more calls are made than
/// responses have been pushed, the last response is reused.
///
/// # Examples
///
/// ```rust
/// use toride_runner::{CommandOutput, CommandSpec, Runner};
/// use toride_runner::fake::FakeRunner;
///
/// let runner = FakeRunner::new()
///     .push_response(CommandOutput::from_stdout("ok"));
///
/// let spec = CommandSpec::new("echo").arg("hello");
/// let output = runner.run(&spec).unwrap();
/// assert_eq!(output.stdout_trimmed(), "ok");
/// ```
pub struct FakeRunner {
    responses: Arc<Mutex<Vec<CommandOutput>>>,
    calls: Arc<Mutex<Vec<CommandSpec>>>,
}

impl FakeRunner {
    /// Create a new `FakeRunner` with no responses configured.
    #[must_use]
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a response to the queue.
    #[must_use]
    pub fn push_response(self, output: CommandOutput) -> Self {
        self.responses.lock().expect("responses lock").push(output);
        self
    }

    /// Return a snapshot of all calls made to this runner.
    pub fn calls(&self) -> Vec<CommandSpec> {
        self.calls.lock().expect("calls lock").clone()
    }
}

impl Default for FakeRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl Runner for FakeRunner {
    fn run(&self, spec: &CommandSpec) -> Result<CommandOutput> {
        // Record the call.
        self.calls.lock().expect("calls lock").push(spec.clone());

        let mut responses = self.responses.lock().expect("responses lock");
        if responses.is_empty() {
            // Default: return a successful empty output.
            return Ok(CommandOutput::from_stdout(String::new()));
        }

        if responses.len() == 1 {
            // Reuse the last response.
            return Ok(responses.first().expect("just checked").clone());
        }

        // Pop the next response (FIFO).
        Ok(responses.remove(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_pushed_response() {
        let runner = FakeRunner::new().push_response(CommandOutput::from_stdout("first"));
        let output = runner.run(&CommandSpec::new("cmd")).unwrap();
        assert_eq!(output.stdout_trimmed(), "first");
    }

    #[test]
    fn records_calls() {
        let runner = FakeRunner::new().push_response(CommandOutput::from_stdout("ok"));
        let _ = runner.run(&CommandSpec::new("a").arg("1"));
        let _ = runner.run(&CommandSpec::new("b").arg("2"));

        let calls = runner.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].program, "a");
        assert_eq!(calls[1].program, "b");
    }

    #[test]
    fn fifo_ordering() {
        let runner = FakeRunner::new()
            .push_response(CommandOutput::from_stdout("1"))
            .push_response(CommandOutput::from_stdout("2"));

        let o1 = runner.run(&CommandSpec::new("cmd")).unwrap();
        let o2 = runner.run(&CommandSpec::new("cmd")).unwrap();
        assert_eq!(o1.stdout_trimmed(), "1");
        assert_eq!(o2.stdout_trimmed(), "2");
    }

    #[test]
    fn default_response_when_empty() {
        let runner = FakeRunner::new();
        let output = runner.run(&CommandSpec::new("cmd")).unwrap();
        assert!(output.success);
        assert!(output.stdout.is_empty());
    }

    #[test]
    fn single_response_reused() {
        let runner = FakeRunner::new().push_response(CommandOutput::from_stdout("only"));
        let _ = runner.run(&CommandSpec::new("a"));
        let _ = runner.run(&CommandSpec::new("b"));
        let _ = runner.run(&CommandSpec::new("c"));
        // Single response is reused, not consumed.
        assert_eq!(runner.calls().len(), 3);
    }
}
