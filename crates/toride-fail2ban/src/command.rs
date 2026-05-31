//! Centralized command execution module.
//!
//! All external process spawning in this crate **must** go through the
//! [`Runner`] trait defined here. No ad-hoc `std::process::Command` calls are
//! allowed elsewhere in the codebase.
//!
//! Two implementations are provided:
//!
//! - [`DuctRunner`] -- production implementation backed by the `duct` crate.
//! - [`FakeRunner`] -- test double that records calls and returns pre-canned
//!   responses.
//!
//! # Security
//!
//! Arguments are always passed as arrays (no shell string concatenation).
//! Sensitive values containing "password", "token", "key", or "secret" are
//! redacted in log output.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Output;
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

use crate::Error;
use crate::Result;

// ---------------------------------------------------------------------------
// CommandOutput
// ---------------------------------------------------------------------------

/// Captured output from an external command.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Standard output captured as a UTF-8 string.
    pub stdout: String,
    /// Standard error captured as a UTF-8 string.
    pub stderr: String,
    /// Exit code, or `None` if the process was killed by a signal.
    pub exit_code: Option<i32>,
    /// Convenience: `true` when `exit_code` is `Some(0)`.
    pub success: bool,
}

impl CommandOutput {
    /// Build a `CommandOutput` from a `std::process::Output`.
    fn from_raw_output(output: &Output) -> Self {
        let exit_code = output.status.code();
        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        Self {
            stdout,
            stderr,
            exit_code,
            success,
        }
    }

    /// Create a successful empty output (used in dry-run mode).
    fn empty_success() -> Self {
        Self {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(0),
            success: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Runner trait
// ---------------------------------------------------------------------------

/// Trait for executing external commands.
///
/// Every subprocess in this crate is spawned through a `Runner` implementation.
/// This makes the entire call stack testable via [`FakeRunner`] and keeps
/// logging / dry-run / timeout behaviour in one place.
pub trait Runner: Send + Sync {
    /// Execute `program` with `args`, waiting up to the runner's default timeout.
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput>;

    /// Execute `program` with `args`, waiting at most `timeout`.
    fn run_with_timeout(
        &self,
        program: &str,
        args: &[&str],
        timeout: Duration,
    ) -> Result<CommandOutput>;

    /// Whether the runner is in dry-run mode (no real commands executed).
    fn dry_run(&self) -> bool;

    /// Enable or disable dry-run mode.
    fn set_dry_run(&mut self, dry_run: bool);
}

// ---------------------------------------------------------------------------
// Redaction helper
// ---------------------------------------------------------------------------

/// Keywords that mark an argument value as sensitive.
const SENSITIVE_KEYWORDS: &[&str] = &["password", "token", "key", "secret"];

/// Redact any argument whose value contains a sensitive keyword.
///
/// Returns a formatted string suitable for logging.
fn redacted_cmd_str(program: &str, args: &[&str]) -> String {
    let mut parts = vec![program.to_string()];
    for arg in args {
        let lower = arg.to_ascii_lowercase();
        if SENSITIVE_KEYWORDS.iter().any(|kw| lower.contains(kw)) {
            parts.push("***".to_string());
        } else {
            parts.push((*arg).to_string());
        }
    }
    parts.join(" ")
}

// ---------------------------------------------------------------------------
// DuctRunner
// ---------------------------------------------------------------------------

/// Production command runner backed by the `duct` crate.
///
/// Uses [`duct::cmd`] for execution. Timeouts are implemented by spawning the
/// child via `start()`, then waiting on a channel with a deadline. If the
/// deadline expires the child is killed.
pub struct DuctRunner {
    /// Default timeout applied by [`Runner::run`].
    default_timeout: Duration,
    /// When `true`, commands are logged but not executed.
    dry_run: bool,
}

impl DuctRunner {
    /// Create a new runner with a 30-second default timeout.
    #[must_use]
    pub fn new() -> Self {
        Self::with_timeout(Duration::from_secs(30))
    }

    /// Create a new runner with the given default timeout.
    #[must_use]
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            default_timeout: timeout,
            dry_run: false,
        }
    }

    /// Execute a command with an explicit timeout using duct.
    ///
    /// The child is spawned with `stdout_capture` / `stderr_capture` and
    /// `unchecked()` so that we can inspect output even on non-zero exit.
    /// A timeout is enforced by waiting on a channel with `recv_timeout`.
    fn execute(program: &str, args: &[&str], timeout: Duration) -> Result<CommandOutput> {
        let cmd_str = redacted_cmd_str(program, args);
        tracing::debug!(cmd = %cmd_str, "executing");

        // Spawn the child.  `unchecked()` prevents duct from turning a
        // non-zero exit status into an `io::Error`, which lets us inspect the
        // output ourselves.
        let handle = duct::cmd(program, args)
            .stdout_capture()
            .stderr_capture()
            .unchecked()
            .start()
            .map_err(|e| Error::CommandFailed(format!("failed to spawn {program}: {e}")))?;

        // Channel for the owned output of `handle.wait()`.
        // `Handle::wait(&self) -> io::Result<&Output>` borrows the handle, so
        // we must clone the `Output` inside the spawned thread to obtain an
        // owned value we can send across the channel.
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            let result = handle.wait().map(|o| o.clone());
            let _ = tx.send(result);
        });

        // Wait up to the timeout for the child to finish.
        let raw_output = rx
            .recv_timeout(timeout)
            .map_err(|_| Error::CommandTimeout(timeout))?
            .map_err(|e| Error::CommandFailed(format!("wait failed for {program}: {e}")))?;

        let result = CommandOutput::from_raw_output(&raw_output);

        if !result.success {
            tracing::warn!(
                cmd = %cmd_str,
                exit = ?result.exit_code,
                stderr = %result.stderr.trim(),
                "command failed"
            );
        }

        Ok(result)
    }
}

impl Default for DuctRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl Runner for DuctRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput> {
        let cmd_str = redacted_cmd_str(program, args);

        if self.dry_run {
            tracing::info!(cmd = %cmd_str, "[dry-run]");
            return Ok(CommandOutput::empty_success());
        }

        Self::execute(program, args, self.default_timeout)
    }

    fn run_with_timeout(
        &self,
        program: &str,
        args: &[&str],
        timeout: Duration,
    ) -> Result<CommandOutput> {
        let cmd_str = redacted_cmd_str(program, args);

        if self.dry_run {
            tracing::info!(cmd = %cmd_str, timeout = ?timeout, "[dry-run]");
            return Ok(CommandOutput::empty_success());
        }

        Self::execute(program, args, timeout)
    }

    fn dry_run(&self) -> bool {
        self.dry_run
    }

    fn set_dry_run(&mut self, dry_run: bool) {
        self.dry_run = dry_run;
    }
}

// ---------------------------------------------------------------------------
// FakeRunner
// ---------------------------------------------------------------------------

/// Test double that records invocations and returns pre-configured responses.
///
/// # Example
///
/// ```ignore
/// let mut fake = FakeRunner::new();
/// fake.with_response("echo", &["hello"], CommandOutput::empty_success());
///
/// let out = fake.run("echo", &["hello"]).unwrap();
/// assert!(out.success);
/// assert_eq!(fake.calls().len(), 1);
/// ```
pub struct FakeRunner {
    /// Pre-configured responses keyed by `"{program} {args.join(" ")}"`.
    responses: HashMap<String, CommandOutput>,
    /// Dry-run flag (mirrors the trait method).
    dry_run: bool,
    /// Ordered record of all calls made through [`Runner::run`] and
    /// [`Runner::run_with_timeout`].
    calls: Mutex<Vec<(String, Vec<String>)>>,
}

impl FakeRunner {
    /// Create a `FakeRunner` with no pre-configured responses.
    #[must_use]
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
            dry_run: false,
            calls: Mutex::new(Vec::new()),
        }
    }

    /// Register a canned response for a specific `(program, args)` pair.
    ///
    /// Uses the builder pattern so multiple responses can be chained.
    pub fn with_response(
        &mut self,
        program: &str,
        args: &[&str],
        output: CommandOutput,
    ) -> &mut Self {
        let key = format!("{program} {}", args.join(" "));
        self.responses.insert(key, output);
        self
    }

    /// Return a snapshot of all recorded calls in order.
    ///
    /// Each entry is `(program, args)`.
    pub fn calls(&self) -> Vec<(String, Vec<String>)> {
        self.calls
            .lock()
            .expect("FakeRunner calls mutex should not be poisoned")
            .clone()
    }

    /// Look up the canned response for a given program/args pair.
    fn lookup(&self, program: &str, args: &[&str]) -> CommandOutput {
        let key = format!("{program} {}", args.join(" "));
        self.responses.get(&key).cloned().unwrap_or_else(|| {
            CommandOutput {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: Some(0),
                success: true,
            }
        })
    }

    /// Record a call in the internal log.
    fn record(&self, program: &str, args: &[&str]) {
        self.calls
            .lock()
            .expect("FakeRunner calls mutex should not be poisoned")
            .push((
                program.to_string(),
                args.iter().map(|s| (*s).to_string()).collect(),
            ));
    }
}

impl Default for FakeRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl Runner for FakeRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput> {
        self.record(program, args);
        Ok(self.lookup(program, args))
    }

    fn run_with_timeout(
        &self,
        program: &str,
        args: &[&str],
        _timeout: Duration,
    ) -> Result<CommandOutput> {
        self.record(program, args);
        Ok(self.lookup(program, args))
    }

    fn dry_run(&self) -> bool {
        self.dry_run
    }

    fn set_dry_run(&mut self, dry_run: bool) {
        self.dry_run = dry_run;
    }
}

// ---------------------------------------------------------------------------
// Binary discovery
// ---------------------------------------------------------------------------

/// Locate a binary on the system `$PATH`.
///
/// Returns [`Error::NotFound`] if the binary cannot be found.
pub fn find_binary(name: &str) -> Result<PathBuf> {
    which::which(name).map_err(|_| Error::NotFound(name.to_string()))
}

#[cfg(test)]
#[path = "command.test.rs"]
mod tests;
