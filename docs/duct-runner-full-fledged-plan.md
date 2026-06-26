# Duct Runner Full-Fledged Plan

## Goal

Make `toride-runner::DuctRunner` a full synchronous process runner, not just a minimal captured-output wrapper. It should remain compatible with the existing `Runner` trait and `CommandSpec`, but support the execution controls, safety guarantees, diagnostics, and parity tests needed for production use across Toride crates.

## Current State

After the current implementation pass, `crates/toride-runner/src/duct_runner.rs`
supports:

- program and argv execution through `duct::cmd`
- optional cwd
- additive environment variables
- string stdin
- captured stdout and stderr through `OutputMode::Capture`
- inherited parent stdio through `OutputMode::Inherit`
- explicit rejection of `OutputMode::Stream` for sync Duct execution
- non-zero exits returned as `CommandOutput`
- default 60 second timeout when `CommandSpec::timeout` is absent, unless a
  configured runner disables the fallback timeout
- configurable fallback timeout and command logging through
  `DuctRunnerOptions`
- structured spawn, wait, timeout, and checked-command errors
- redacted checked-command failure arguments when `CommandSpec::redact` is true
- timeout cleanup that kills and reaps the Duct handle, with warning logs for
  kill/reap failures
- focused tests for cwd, failures, redaction, timeout metadata, timeout cleanup,
  stdout/stderr separation, large output, output modes, options, and serde
  compatibility

This is stronger than the initial minimal wrapper, but it is not a complete
runner abstraction yet.

## Implementation Status

- Phase 1 is implemented: Duct spawn/wait errors are mapped to structured error variants, command completion logs include redacted display strings and elapsed time, `run_checked()` uses redacted args when requested, and the Duct test suite now covers cwd, checked failure, redaction, spawn failure, timeout metadata, timeout cleanup, stdout/stderr separation, large output, and exit-code preservation.
- Phase 2 is implemented: `DuctRunnerOptions`, `DuctRunnerBuilder`, and `ConfiguredDuctRunner` provide configurable fallback timeouts, no-default-timeout mode, and command logging control while preserving `DuctRunner` unit-struct usage.
- Phase 3 is implemented: `CommandSpec` now carries `OutputMode` with serde backward compatibility, `DuctRunner` honors `Capture` and `Inherit`, `Stream` is explicitly unsupported for sync Duct execution, and `FakeRunner` exact matching includes output mode.
- Phases 4-7 remain open: environment policy, output limits/raw bytes,
  process-tree cleanup policy, expanded parity coverage, and docs/examples.

## Progress Audit

Audit date: 2026-06-26.

The plan was checked against the current code three ways:

- API surface: `CommandSpec`, `DuctRunner`, `ConfiguredDuctRunner`,
  `DuctRunnerOptions`, `OutputMode`, and `FakeRunner` exact matching were
  compared against the planned phases.
- Behavior: the current Duct implementation was checked for timeout defaults,
  spawn/wait mapping, checked failure redaction, output-mode handling, and
  timeout cleanup behavior.
- Tests: runner and sidebar tests were re-run after the plan update before
  committing.

Completed during the current pass:

- configurable fallback timeout
- no-default-timeout mode
- runner-level options and builder
- command logging control
- `CommandSpec::output_mode`
- serde backward compatibility for specs without `output_mode`
- `OutputMode::Capture` and `OutputMode::Inherit` in DuctRunner
- explicit unsupported error for `OutputMode::Stream`
- fake-runner exact matching for output mode
- `Error::SpawnFailed` and `Error::WaitFailed` mapping
- redacted `run_checked()` failure arguments
- duration and redacted command display in completion logs
- timeout kill/reap warning logs
- focused DuctRunner tests for the above behavior

## Remaining Gaps

### Execution Policy

- No option to merge stderr into stdout.
- No option to suppress stdout/stderr.
- No explicit shell mode. This is good by default, but there is no deliberate escape hatch for shell syntax when a caller truly needs it.

### Environment Control

- Only additive env vars are supported.
- No clean environment mode.
- No env removal support.
- No documented policy for preserving required platform env vars.
- Fake runner exact-match comparison still ignores timeout and redaction. That
  is currently intentional for timeout because it is a runtime policy concern,
  but it should be re-audited when environment policy and additional
  `CommandSpec` fields are added.

### Error Semantics

- Stdin setup/write failures are not distinguished.
- Timeout diagnostics include program, args, and duration, but there is no
  richer structured context such as cwd, output mode, or sanitized env summary.

### Process Cleanup

- Timeout cleanup relies on `handle.kill()` plus `handle.wait()`, but the
  platform and process-tree guarantees are not documented in crate docs.
- No explicit process-group or job-object policy.
- Tests cover a direct timed-out command, but not grandchild survival behavior.
- No platform-specific notes for Unix vs Windows cleanup behavior.

### Output Handling

- Output is lossy UTF-8 only (`String::from_utf8_lossy`).
- `CommandOutput` cannot preserve raw bytes.
- No output size limit to guard against commands producing unbounded data.
- No streaming support for sync callers beyond the current explicit unsupported
  error for `OutputMode::Stream`.
- No line/event callback for progress output.
- No output redirection to file.

### Diagnostics and Redaction

- Env display redaction exists but is not integrated into runner logs.
- No tracing span around command execution.
- Completion logs include redacted command display, program, exit code, and
  duration, but failures do not yet share a common tracing span or sanitized env
  summary.

### Parity With TokioRunner

- Basic parity tests exist, but coverage is thin.
- Some focused DuctRunner tests cover spawn error classification, timeout
  metadata, specific non-zero exit codes, and stdout/stderr separation with
  larger output. Shared Duct/Tokio parity coverage still needs to be expanded.
- No parity for cwd resolution edge cases.

### API Shape

- `DuctRunner` remains a unit struct for compatibility, with configured behavior
  available through `ConfiguredDuctRunner`.
- `CommandSpec` lacks fields for clean env, env removal, byte stdin, output
  limits, and shell opt-in.
- Adding fields to `CommandSpec` affects serde compatibility and fake-runner exact matching.

## Target Behavior

### Minimal Compatibility Target

Existing code should keep working:

```rust
let runner = DuctRunner;
let output = runner.run(&CommandSpec::new("echo").arg("ok"))?;
```

The unit struct can remain as the default runner. Advanced behavior should be available through a configuration type or constructor, for example:

```rust
let runner = DuctRunner::builder()
    .default_timeout(Duration::from_secs(10))
    .log_commands(true)
    .build();
```

`DuctRunner` remains a unit struct for compatibility. Configured behavior lives
in `ConfiguredDuctRunner`, built through `DuctRunner::builder()` or
`DuctRunner::with_options(options)`.

### CommandSpec Extensions

Add only when needed, and preserve serde backward compatibility:

- `env_remove: Vec<String>`
- `clear_env: bool`
- `stdin_bytes: Option<Vec<u8>>` or a `CommandStdin` enum
- `timeout_policy: TimeoutPolicy` if `None` must distinguish “use runner default” from “no timeout”
- `output_limit: Option<usize>`
- `shell: Option<ShellSpec>` for deliberate shell execution

Recommended rule: `CommandSpec` describes command intent; `DuctRunnerOptions` describes operational defaults and safety policy.

### Error Mapping

Map errors consistently:

- spawn error -> `Error::SpawnFailed`
- wait error -> `Error::WaitFailed`
- stdin setup/write error -> `Error::StdinFailed`
- timeout -> `Error::CommandTimeout`
- non-zero exit from `run()` -> successful `CommandOutput`
- non-zero exit from `run_checked()` -> `Error::CommandFailed`

`run_checked()` uses redacted display args in error messages when `spec.redact`
is true.

### Output Modes

Support at least:

- `Capture`: current behavior, captured stdout/stderr
- `Inherit`: child inherits parent stdio, returned `CommandOutput` has empty stdout/stderr and exit code
- `Stream`: currently unsupported for sync Duct with a clear error; future work
  may add a blocking event sink/callback trait separate from async streaming

Do not silently treat `Stream` as `Capture`; that hides caller intent.

### Cleanup Policy

Document and test:

- direct child is killed on timeout
- pipes are closed
- process is reaped where possible
- process-tree cleanup is not guaranteed unless explicitly implemented

If process-tree cleanup becomes required, add a platform-specific process group mode:

- Unix: spawn in a new process group and signal the group on timeout
- Windows: use job objects or document direct-child-only cleanup

## Implementation Phases

### Phase 1: Harden Current DuctRunner

Keep public API stable.

Status: implemented.

- Map spawn errors to `Error::SpawnFailed`.
- Map wait errors to `Error::WaitFailed`.
- Add cwd test matching Tokio’s cwd test.
- Add `run_checked()` failure test.
- Add specific exit-code preservation test.
- Add stdout/stderr separation test.
- Add large-output capture test.
- Add timeout metadata test.
- Add timeout-kills-child test.
- Use `display_command()` in tracing logs.
- Redact args in `run_checked()` failure errors when `spec.redact` is true.

Exit criteria:

- Existing tests pass.
- New DuctRunner tests pass.
- Basic parity tests still pass.

### Phase 2: Add Runner Options

Introduce configuration without forcing all callers to change.

Status: implemented.

- Add `DuctRunnerOptions`.
- Add `DuctRunner::builder()` or `DuctRunner::with_options(options)`.
- Support configurable default timeout.
- Support no default timeout.
- Support command logging on/off.
- Preserve `DuctRunner` default behavior.

Exit criteria:

- Existing unit-struct usage still compiles.
- Tests prove default timeout and no-default-timeout behavior.

### Phase 3: Wire OutputMode

Make `OutputMode` meaningful for sync execution.

Status: implemented.

- Add `output_mode` to `CommandSpec` with serde defaults.
- Implement `Capture`.
- Implement `Inherit`.
- Decide whether sync `Stream` is unsupported or backed by a sync streaming trait.
- Update fake runner matching to include `output_mode`.
- Add tests for inherited stdio exit code and empty captured output.

Exit criteria:

- `OutputMode::Capture` preserves current behavior.
- `OutputMode::Inherit` behaves predictably.
- `OutputMode::Stream` has explicit behavior.

### Phase 4: Environment Policy

Add deliberate environment control.

- Add `env_remove`.
- Add `clear_env`.
- Decide whether `clear_env` preserves minimal platform env vars.
- Implement through Duct’s `env_remove` and `full_env`.
- Update fake runner exact matching.
- Add tests for env removal and clean environment.

Exit criteria:

- Callers can add, remove, and clear env vars intentionally.
- Behavior is documented for Unix/macOS/Windows.

### Phase 5: Output Safety

Prevent pathological command output from destabilizing the app.

- Add output byte limits.
- Decide whether exceeding the limit is an error or truncated output with metadata.
- Consider adding raw-byte output support if commands need non-UTF-8 data.
- Add tests for large output and limit behavior.

Exit criteria:

- No unbounded memory growth for captured commands when callers opt into limits.
- Non-UTF-8 behavior is explicit.

### Phase 6: Process-Tree Cleanup

Only do this if direct-child cleanup is insufficient for real Toride commands.

- Audit commands that spawn grandchildren.
- Add process-group/job-object mode behind options.
- Add platform-specific tests where practical.
- Document guarantees and limitations.

Exit criteria:

- Timeout cleanup guarantees are stronger than direct-child-only.
- Tests prove grandchildren do not survive where supported.

### Phase 7: Full Parity and Documentation

- Expand `parity_tests.rs` for the shared feature set.
- Document where DuctRunner and TokioRunner intentionally differ.
- Add examples for capture, inherited stdio, custom timeout, clean env, redaction, and checked execution.
- Update crate-level docs.

Exit criteria:

- Users can choose Duct vs Tokio based on documented tradeoffs.
- The sync runner no longer has hidden behavior gaps.

## Recommended Next PR

Phases 1-3 are implemented. The next highest-value slice is Phase 4:
environment policy. It is the remaining gap most likely to affect real command
correctness, reproducibility, and testability.

Suggested files for Phase 4:

- `crates/toride-runner/src/duct_runner.rs`
- `crates/toride-runner/src/spec.rs`
- `crates/toride-runner/src/fake.rs`
- `crates/toride-runner/src/parity_tests.rs`

Suggested tests:

- DuctRunner applies additive env vars with existing behavior unchanged.
- DuctRunner removes selected env vars.
- DuctRunner can clear the inherited environment.
- Clean-env behavior preserves or documents required platform variables.
- FakeRunner exact matching handles any new environment-policy fields.
- Serde round trips new `CommandSpec` fields and defaults older JSON safely.

Do not combine Phase 4 with raw-byte output or process-tree cleanup. Those are
separate risk areas and should be reviewed independently.

## Definition of Done

The Duct wrapper is full-fledged when:

- `DuctRunner` has configurable execution policy.
- `CommandSpec` can express output mode and environment policy.
- errors are structured and consistent with TokioRunner.
- sensitive args/env are redacted in diagnostics and checked errors.
- timeout cleanup behavior is documented and tested.
- output handling has clear capture, inherit, and streaming/unsupported-stream semantics.
- parity tests cover all shared Duct/Tokio behavior.
- crate docs explain capability boundaries clearly.
