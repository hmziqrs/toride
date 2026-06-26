//! Real command execution via the `duct` crate.
//!
//! [`DuctRunner`] is the production implementation of [`Runner`](crate::Runner).
//! It spawns subprocesses, captures stdout/stderr, and respects timeouts.

use std::time::{Duration, Instant};

use crate::error::{Error, Result};
use crate::output::CommandOutput;
use crate::output_mode::OutputMode;
use crate::runner::Runner;
use crate::spec::CommandSpec;

/// Default command timeout in seconds when none is specified.
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Runtime policy for [`DuctRunner`] command execution.
///
/// These are runner defaults. A timeout set directly on [`CommandSpec`] still
/// wins for that individual command.
#[derive(Debug, Clone)]
pub struct DuctRunnerOptions {
    /// Timeout applied when [`CommandSpec::timeout`] is absent.
    ///
    /// `None` means commands without an explicit timeout can run until they
    /// exit naturally.
    pub default_timeout: Option<Duration>,
    /// Whether completed commands emit debug tracing logs.
    pub log_commands: bool,
}

impl Default for DuctRunnerOptions {
    fn default() -> Self {
        Self {
            default_timeout: Some(Duration::from_secs(DEFAULT_TIMEOUT_SECS)),
            log_commands: true,
        }
    }
}

/// Builder for configured Duct-backed runners.
#[derive(Debug, Clone)]
pub struct DuctRunnerBuilder {
    options: DuctRunnerOptions,
}

impl DuctRunnerBuilder {
    /// Set the fallback timeout for specs that do not define one.
    pub fn default_timeout(mut self, timeout: Duration) -> Self {
        self.options.default_timeout = Some(timeout);
        self
    }

    /// Disable the fallback timeout for specs that do not define one.
    pub fn no_default_timeout(mut self) -> Self {
        self.options.default_timeout = None;
        self
    }

    /// Enable or disable completion debug logs.
    pub fn log_commands(mut self, enabled: bool) -> Self {
        self.options.log_commands = enabled;
        self
    }

    /// Build a configured Duct runner.
    pub fn build(self) -> ConfiguredDuctRunner {
        ConfiguredDuctRunner {
            options: self.options,
        }
    }
}

/// A [`Runner`] implementation that executes commands via the `duct` crate.
///
/// # Examples
///
/// ```rust,ignore
/// use toride_runner::{CommandSpec, DuctRunner, Runner};
///
/// let runner = DuctRunner;
/// let spec = CommandSpec::new("echo").arg("hello");
/// let output = runner.run(&spec)?;
/// assert!(output.success);
/// assert_eq!(output.stdout_trimmed(), "hello");
/// ```
pub struct DuctRunner;

impl Runner for DuctRunner {
    fn run(&self, spec: &CommandSpec) -> Result<CommandOutput> {
        run_duct_command(spec, &DuctRunnerOptions::default())
    }
}

impl DuctRunner {
    /// Start building a configured Duct runner.
    pub fn builder() -> DuctRunnerBuilder {
        DuctRunnerBuilder {
            options: DuctRunnerOptions::default(),
        }
    }

    /// Create a configured Duct runner from explicit options.
    pub fn with_options(options: DuctRunnerOptions) -> ConfiguredDuctRunner {
        ConfiguredDuctRunner { options }
    }
}

/// A Duct-backed runner with explicit execution options.
#[derive(Debug, Clone)]
pub struct ConfiguredDuctRunner {
    options: DuctRunnerOptions,
}

impl ConfiguredDuctRunner {
    /// Return this runner's execution options.
    #[must_use]
    pub fn options(&self) -> &DuctRunnerOptions {
        &self.options
    }
}

impl Runner for ConfiguredDuctRunner {
    fn run(&self, spec: &CommandSpec) -> Result<CommandOutput> {
        run_duct_command(spec, &self.options)
    }
}

fn run_duct_command(spec: &CommandSpec, options: &DuctRunnerOptions) -> Result<CommandOutput> {
    if spec.output_mode == OutputMode::Stream {
        return Err(Error::Other(
            "OutputMode::Stream is not supported by the synchronous DuctRunner; use TokioRunner with the stream feature".to_owned(),
        ));
    }

    let started_at = Instant::now();
    let displayed = crate::display::display_command(spec, &[]);
    let mut cmd = duct::cmd(&spec.program, &spec.args);

    // Apply working directory if specified.
    if let Some(ref cwd) = spec.cwd {
        cmd = cmd.dir(cwd);
    }

    cmd = apply_env_policy(cmd, spec);

    // Pipe stdin data if provided.
    if let Some(ref stdin_data) = spec.stdin {
        cmd = cmd.stdin_bytes(stdin_data.as_bytes());
    }

    let timeout = spec.timeout.or(options.default_timeout);

    // Spawn with stdout/stderr capture by default. Inherit mode deliberately
    // connects child output to the parent and returns empty captured strings.
    if spec.output_mode == OutputMode::Capture {
        cmd = cmd.stdout_capture().stderr_capture();
    }

    // Use unchecked so non-zero exit
    // does not immediately error — we capture it in CommandOutput.
    let handle = cmd.unchecked().start().map_err(|e| Error::SpawnFailed {
        program: spec.program.clone(),
        detail: e.to_string(),
    })?;

    let output = if let Some(timeout) = timeout {
        match handle.wait_timeout(timeout) {
            Ok(Some(output)) => output.clone(),
            Ok(None) => {
                // Timeout expired. Duct kills all processes it started for an
                // expression; process-tree guarantees beyond that are tracked
                // in docs/duct-runner-full-fledged-plan.md.
                if let Err(err) = handle.kill() {
                    tracing::warn!(
                        command = %displayed,
                        error = %err,
                        "failed to kill timed-out command"
                    );
                }
                if let Err(err) = handle.wait() {
                    tracing::warn!(
                        command = %displayed,
                        error = %err,
                        "failed to reap timed-out command"
                    );
                }
                return Err(Error::CommandTimeout {
                    program: spec.program.clone(),
                    args: spec.args.clone(),
                    timeout,
                });
            }
            Err(e) => return Err(wait_failed(spec, e)),
        }
    } else {
        handle.wait().map_err(|e| wait_failed(spec, e))?.clone()
    };

    let stdout = if spec.output_mode == OutputMode::Capture {
        String::from_utf8_lossy(&output.stdout).into_owned()
    } else {
        String::new()
    };
    let stderr = if spec.output_mode == OutputMode::Capture {
        String::from_utf8_lossy(&output.stderr).into_owned()
    } else {
        String::new()
    };
    let exit_code = output.status.code();

    if options.log_commands {
        tracing::debug!(
            command = %displayed,
            program = %spec.program,
            exit_code = ?exit_code,
            elapsed_ms = started_at.elapsed().as_millis(),
            "command completed"
        );
    }

    Ok(CommandOutput::new(stdout, stderr, exit_code))
}

fn wait_failed(spec: &CommandSpec, error: std::io::Error) -> Error {
    Error::WaitFailed {
        program: spec.program.clone(),
        detail: error.to_string(),
    }
}

fn apply_env_policy(mut cmd: duct::Expression, spec: &CommandSpec) -> duct::Expression {
    if spec.clear_env {
        let mut env = clean_env_values(spec);
        env.extend(spec.env.iter().cloned());
        return cmd.full_env(env);
    }

    for (key, value) in &spec.env {
        cmd = cmd.env(key, value);
    }

    for key in &spec.env_remove {
        if !spec
            .env
            .iter()
            .any(|(env_key, _)| env_key_matches(env_key, key))
        {
            cmd = cmd.env_remove(key);
        }
    }

    cmd
}

fn clean_env_values(spec: &CommandSpec) -> Vec<(String, String)> {
    platform_env_preserved_for_clean_env()
        .into_iter()
        .filter(|(key, _)| {
            !spec
                .env_remove
                .iter()
                .any(|removed| env_key_matches(removed, key))
                || spec
                    .env
                    .iter()
                    .any(|(env_key, _)| env_key_matches(env_key, key))
        })
        .collect()
}

#[cfg(windows)]
fn platform_env_preserved_for_clean_env() -> Vec<(String, String)> {
    ["SystemRoot", "SystemDrive", "WINDIR"]
        .into_iter()
        .filter_map(|key| std::env::var(key).ok().map(|value| (key.to_owned(), value)))
        .collect()
}

#[cfg(not(windows))]
fn platform_env_preserved_for_clean_env() -> Vec<(String, String)> {
    Vec::new()
}

#[cfg(windows)]
fn env_key_matches(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

#[cfg(not(windows))]
fn env_key_matches(a: &str, b: &str) -> bool {
    a == b
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn echo_hello() {
        let runner = DuctRunner;
        let spec = CommandSpec::new("echo").arg("hello");
        let output = runner.run(&spec).unwrap();
        assert!(output.success);
        assert_eq!(output.stdout_trimmed(), "hello");
    }

    #[test]
    fn failed_command() {
        let runner = DuctRunner;
        let spec = CommandSpec::new("false");
        let output = runner.run(&spec).unwrap();
        assert!(!output.success);
    }

    #[test]
    fn timeout_expires() {
        let runner = DuctRunner;
        let spec = CommandSpec::new("sleep")
            .arg("10")
            .timeout(Duration::from_millis(50));
        let result = runner.run(&spec);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::CommandTimeout { .. }));
    }

    #[test]
    fn stdin_piped() {
        let runner = DuctRunner;
        let spec = CommandSpec::new("cat").stdin("piped content");
        let output = runner.run(&spec).unwrap();
        assert_eq!(output.stdout_trimmed(), "piped content");
    }

    #[test]
    fn env_passed() {
        let runner = DuctRunner;
        let spec = CommandSpec::new("env").env("TORIDE_TEST_VAR", "42");
        let output = runner.run(&spec).unwrap();
        assert!(output.stdout.contains("TORIDE_TEST_VAR=42"));
    }

    #[test]
    fn env_remove_unsets_inherited_variable() {
        let runner = DuctRunner;
        let spec = CommandSpec::new("/bin/sh")
            .args(["-c", "printf '%s' \"${HOME-unset}\""])
            .env_remove("HOME");
        let output = runner.run(&spec).unwrap();

        assert_eq!(output.stdout, "unset");
    }

    #[test]
    fn explicit_env_wins_over_env_remove() {
        let runner = DuctRunner;
        let spec = CommandSpec::new("/bin/sh")
            .args(["-c", "printf '%s' \"${TORIDE_REMOVE_ME-unset}\""])
            .env_remove("TORIDE_REMOVE_ME")
            .env("TORIDE_REMOVE_ME", "present");
        let output = runner.run(&spec).unwrap();

        assert_eq!(output.stdout, "present");
    }

    #[test]
    fn clear_env_removes_inherited_variables() {
        let runner = DuctRunner;
        let spec = CommandSpec::new("/bin/sh")
            .args(["-c", "printf '%s:%s' \"${HOME-unset}\" \"$TORIDE_ONLY\""])
            .clear_env(true)
            .env("TORIDE_ONLY", "kept");
        let output = runner.run(&spec).unwrap();

        assert_eq!(output.stdout, "unset:kept");
    }

    #[test]
    fn cwd_applied() {
        let runner = DuctRunner;
        let spec = CommandSpec::new("pwd").cwd("/tmp");
        let output = runner.run(&spec).unwrap();
        let resolved = std::path::Path::new("/tmp")
            .canonicalize()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "/tmp".to_owned());
        assert_eq!(output.stdout_trimmed(), resolved);
    }

    #[test]
    fn run_checked_errors_on_failure() {
        let runner = DuctRunner;
        let spec = CommandSpec::new("false");
        let result = runner.run_checked(&spec);
        assert!(matches!(result.unwrap_err(), Error::CommandFailed { .. }));
    }

    #[test]
    fn run_checked_redacts_args_when_requested() {
        let runner = DuctRunner;
        let spec = CommandSpec::new("bash")
            .args(["-c", "exit 7", "--token", "secret-value"])
            .redact(true);
        let result = runner.run_checked(&spec);

        match result.unwrap_err() {
            Error::CommandFailed { args, .. } => {
                assert!(args.contains("***"));
                assert!(!args.contains("secret-value"));
            }
            other => panic!("expected CommandFailed, got {other:?}"),
        }
    }

    #[test]
    fn spawn_failed() {
        let runner = DuctRunner;
        let spec = CommandSpec::new("definitely_not_a_real_binary_xyz_123");
        let result = runner.run(&spec);
        assert!(matches!(result.unwrap_err(), Error::SpawnFailed { .. }));
    }

    #[test]
    fn timeout_error_metadata() {
        let runner = DuctRunner;
        let timeout = Duration::from_millis(50);
        let spec = CommandSpec::new("sleep").arg("10").timeout(timeout);
        let result = runner.run(&spec);

        match result.unwrap_err() {
            Error::CommandTimeout {
                program,
                args,
                timeout: reported,
            } => {
                assert_eq!(program, "sleep");
                assert_eq!(args, vec!["10"]);
                assert_eq!(reported, timeout);
            }
            other => panic!("expected CommandTimeout, got {other:?}"),
        }
    }

    #[test]
    fn stdout_stderr_separation() {
        let runner = DuctRunner;
        let spec = CommandSpec::new("bash").args(["-c", "echo OUT; echo ERR >&2"]);
        let output = runner.run(&spec).unwrap();

        assert!(output.success);
        assert!(output.stdout.contains("OUT"));
        assert!(output.stderr.contains("ERR"));
        assert!(!output.stdout.contains("ERR"));
        assert!(!output.stderr.contains("OUT"));
    }

    #[test]
    fn large_output_captured() {
        let runner = DuctRunner;
        let spec = CommandSpec::new("bash")
            .args(["-c", "for i in $(seq 1 100); do echo \"line $i\"; done"]);
        let output = runner.run(&spec).unwrap();

        assert!(output.success);
        let lines: Vec<&str> = output
            .stdout
            .lines()
            .filter(|line| !line.is_empty())
            .collect();
        assert_eq!(lines.len(), 100);
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[99], "line 100");
    }

    #[test]
    fn specific_exit_code_preserved() {
        let runner = DuctRunner;
        let spec = CommandSpec::new("bash").args(["-c", "exit 42"]);
        let output = runner.run(&spec).unwrap();

        assert!(!output.success);
        assert_eq!(output.exit_code, Some(42));
    }

    #[test]
    fn timeout_kills_child_process() {
        let runner = DuctRunner;
        let marker = std::env::temp_dir().join(format!(
            "toride_runner_duct_timeout_{}_{}",
            std::process::id(),
            "marker"
        ));
        let _ = std::fs::remove_file(&marker);
        let script = format!("sleep 10 && echo SURVIVED > {}", marker.display());
        let spec = CommandSpec::new("bash")
            .args(["-c", script.as_str()])
            .timeout(Duration::from_millis(100));

        let result = runner.run(&spec);
        assert!(matches!(result.unwrap_err(), Error::CommandTimeout { .. }));
        std::thread::sleep(Duration::from_millis(200));

        let marker_exists = marker.exists();
        let _ = std::fs::remove_file(&marker);
        assert!(!marker_exists, "timed-out child process kept running");
    }

    #[test]
    fn options_default_preserves_unit_runner_policy() {
        let options = DuctRunnerOptions::default();

        assert_eq!(
            options.default_timeout,
            Some(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        );
        assert!(options.log_commands);
    }

    #[test]
    fn builder_configures_default_timeout() {
        let runner = DuctRunner::builder()
            .default_timeout(Duration::from_millis(50))
            .build();
        let spec = CommandSpec::new("sleep").arg("10");
        let result = runner.run(&spec);

        assert!(matches!(result.unwrap_err(), Error::CommandTimeout { .. }));
    }

    #[test]
    fn builder_can_disable_default_timeout() {
        let runner = DuctRunner::builder()
            .no_default_timeout()
            .log_commands(false)
            .build();
        let spec = CommandSpec::new("bash").args(["-c", "sleep 0.05; echo done"]);
        let output = runner.run(&spec).unwrap();

        assert_eq!(runner.options().default_timeout, None);
        assert!(!runner.options().log_commands);
        assert_eq!(output.stdout_trimmed(), "done");
    }

    #[test]
    fn spec_timeout_overrides_runner_default_timeout() {
        let runner = DuctRunner::builder()
            .default_timeout(Duration::from_millis(50))
            .build();
        let spec = CommandSpec::new("bash")
            .args(["-c", "sleep 0.1; echo done"])
            .timeout(Duration::from_secs(1));
        let output = runner.run(&spec).unwrap();

        assert_eq!(output.stdout_trimmed(), "done");
    }

    #[test]
    fn with_options_builds_configured_runner() {
        let runner = DuctRunner::with_options(DuctRunnerOptions {
            default_timeout: None,
            log_commands: false,
        });

        assert_eq!(runner.options().default_timeout, None);
        assert!(!runner.options().log_commands);
    }

    #[test]
    fn capture_output_mode_is_default_behavior() {
        let runner = DuctRunner;
        let spec = CommandSpec::new("bash")
            .args(["-c", "echo OUT; echo ERR >&2"])
            .output_mode(OutputMode::Capture);
        let output = runner.run(&spec).unwrap();

        assert!(output.stdout.contains("OUT"));
        assert!(output.stderr.contains("ERR"));
    }

    #[test]
    fn inherit_output_mode_returns_empty_captured_output() {
        let runner = DuctRunner;
        let spec = CommandSpec::new("bash")
            .args(["-c", "exit 17"])
            .output_mode(OutputMode::Inherit);
        let output = runner.run(&spec).unwrap();

        assert!(!output.success);
        assert_eq!(output.exit_code, Some(17));
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn stream_output_mode_is_explicitly_unsupported_for_duct_runner() {
        let runner = DuctRunner;
        let spec = CommandSpec::new("echo")
            .arg("hello")
            .output_mode(OutputMode::Stream);
        let result = runner.run(&spec);

        match result.unwrap_err() {
            Error::Other(message) => assert!(message.contains("OutputMode::Stream")),
            other => panic!("expected unsupported stream error, got {other:?}"),
        }
    }
}
