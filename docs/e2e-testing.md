# TUI End-to-End Testing

This document defines how Toride should test the terminal UI as a user would experience it.

Web apps have Playwright and Puppeteer: launch a browser, click, type, wait for visible UI, and assert what the user can see. TUI apps can do the same thing, but the browser is replaced by a pseudo-terminal (PTY), and the DOM is replaced by a parsed terminal screen buffer.

The short answer: yes, we can do real E2E testing for TUI apps.

---

# Why Unit Tests Are Not Enough

Unit tests should cover the reducer, planning rules, validation rules, dependency resolution, dry-run behavior, and install action generation. They are necessary, but they do not prove that the app feels usable.

Examples of problems unit tests will usually miss:

* A focused row is not visually distinguishable.
* `Esc` closes the wrong overlay.
* The app renders correctly at `120x40` but breaks at `80x24`.
* A modal traps focus and cannot be dismissed.
* The selected profile changes internal state, but the visible checklist does not update.
* The terminal is left in raw mode after a panic.
* A loading state never appears because the event loop only redraws after the task finishes.
* Key hints exist in state but are clipped on small terminals.

For Toride, these are product bugs. The app is a guided installer, so the terminal experience is the product surface.

---

# Testing Layers

Toride should use three layers of tests.

## 1. Pure Logic Tests

Target:

* `update(&mut Model, Action) -> Vec<Effect>`
* profile defaults
* module dependency rules
* unsafe-combination warnings
* plan generation
* form validation
* install recipe rendering

These tests do not start a terminal. They should be fast, deterministic, and broad.

Example:

```rust
#[test]
fn dokploy_selects_docker_dependency() {
    let mut model = Model::initial_for_test();

    let effects = update(&mut model, Action::ToggleModule(ModuleId::Dokploy));

    assert!(effects.is_empty());
    assert!(model.modules[&ModuleId::Dokploy].selected);
    assert!(model.modules[&ModuleId::Docker].selected);
}
```

## 2. Headless Render Tests

Target:

* individual widgets
* screen layouts
* responsive terminal sizes
* visual regressions in rendered text

Use Ratatui's `TestBackend` for this layer. Ratatui's current docs describe it as a backend that renders to an in-memory buffer and is intended for integration tests of the entire terminal UI. For lower-level widget tests, prefer rendering directly into a `Buffer` when that is enough.

This layer is not full E2E because it does not run the real binary and does not exercise raw mode, alternate screen, terminal input decoding, subprocess lifetime, or panic restoration. It is still valuable because it is stable and cheap.

Recommended crate:

* `insta` for snapshot testing rendered buffers.

Example shape:

```rust
#[test]
fn profile_screen_renders_at_80x24() {
    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    let model = Model::initial_for_test();

    terminal.draw(|frame| view(frame, &model)).unwrap();

    insta::assert_snapshot!(terminal.backend());
}
```

Use this for exact layout expectations, not full user journeys. Ratatui's snapshot recipe also notes that color assertions are not supported by the simple `insta` snapshot path as of the current docs, so color-sensitive checks should use buffer/cell assertions or the PTY E2E harness.

## 3. PTY E2E Tests

Target:

* real binary startup
* real keyboard input
* focus movement
* overlays
* full workflows
* terminal resize behavior
* alternate screen and raw-mode cleanup
* user-visible status, logs, and errors

This is the TUI equivalent of Playwright.

The test harness should:

1. Build or locate the `toride` binary.
2. Spawn it inside a PTY at a fixed terminal size, for example `100x32`.
3. Wait until expected text appears on the parsed screen.
4. Send real key sequences such as `Tab`, `Enter`, `Esc`, arrows, text input, and paste.
5. Assert visible text, selected rows, progress states, and screen transitions.
6. Capture the terminal buffer when the test fails.
7. Kill the process and verify terminal cleanup.

Conceptual example:

```rust
#[test]
fn user_can_select_basic_profile_and_open_plan() {
    let mut app = TuiSession::spawn(env!("CARGO_BIN_EXE_toride"))
        .size(100, 32)
        .env("TORIDE_E2E", "1")
        .start();

    app.wait_for_text("Profiles");
    app.press("Enter");
    app.wait_for_text("Basic Profile");

    app.press("Enter");
    app.wait_for_text("Review modules");
    app.assert_visible("Docker");
    app.assert_visible("UFW firewall");

    app.press("Ctrl+R");
    app.wait_for_text("Dry run");
    app.assert_visible("No changes have been applied");
}
```

The actual API depends on the selected harness, but this is the behavior we want.

In Cargo integration tests, prefer `env!("CARGO_BIN_EXE_toride")` over a hard-coded `./target/debug/toride` path. Cargo provides that variable for built binary targets, which makes the test work across profiles, workspaces, and target directories.

---

# Recommended Rust Approach

Toride is planned as Rust + Ratatui + Crossterm, so the recommended stack is:

* Unit tests for `update`, validation, and plan generation.
* `ratatui::backend::TestBackend` + `insta` for headless render snapshots.
* PTY-backed E2E tests for complete user workflows.

For the PTY layer, use one of these paths.

## Option A: Use `testty`

`testty` is a Rust-native TUI E2E framework. Its docs describe it as PTY-driven, with semantic assertions over terminal state, using `vt100` parsing and optional VHS screenshot capture.

This is the closest match to Playwright for a Rust TUI because it is designed around launching a real TUI binary and asserting what is visible on the terminal screen.

Pros:

* Rust-native.
* Designed specifically for TUI E2E.
* Uses a real PTY.
* Supports semantic assertions instead of only raw output matching.
* Can capture visual artifacts.

Risks:

* It is newer than the core ecosystem.
* We should pin the version and keep the initial test surface small.

Recommended use:

* Start with 3-5 smoke workflows.
* Run in CI on Linux.
* Keep visual snapshots optional at first.
* Pin the exact crate version in `Cargo.toml` after a local spike. The docs.rs index has recently shown fast-moving `0.6.x` releases, so avoid loose assumptions about API names until implementation time.

## Option B: Build a Small Harness with `portable-pty` + `vt100`

If `testty` is too young or too opinionated, build a thin internal harness.

Pieces:

* `portable-pty` to spawn Toride in a pseudo-terminal. The current `portable-pty` docs show creating a PTY with `native_pty_system()`, `openpty(PtySize { rows, cols, .. })`, spawning a command through the slave, reading from the master, and writing input to the master.
* `vt100` to parse ANSI output into a screen buffer. The current `vt100` docs describe it as parsing a terminal byte stream into an in-memory representation of rendered contents, with cell-level access for text and colors.
* helper methods like `press`, `type_text`, `wait_for_text`, `assert_visible`, `resize`, and `snapshot`.

This is more work, but it gives us control over retries, timeouts, artifacts, and CI behavior.

Recommended only if:

* `testty` cannot handle a needed workflow.
* We need tighter control over snapshots.
* We want to avoid depending on a new E2E framework.

## Option C: Use `expectrl` for Prompt-Like Flows

`expectrl` is useful for automating interactive terminal programs. Its current docs describe it as an `expect`-style library for spawning, controlling, and interacting with terminal process I/O. It is closer to classic `expect`/`pexpect`: spawn a process, wait for patterns, send input.

It can help for CLI prompts, but it is weaker for full-screen TUIs because full-screen apps redraw in-place, use alternate screen, and depend on layout position. For Toride, `expectrl` is acceptable for simple fallback checks, but not the primary E2E harness.

---

# What To Test E2E

The PTY E2E suite should stay small and high-value. It should prove user workflows, not re-test every branch already covered by unit tests.

## Initial Smoke Suite

1. Startup and quit
   * Launch at `100x32`.
   * Assert the Profiles screen appears.
   * Press `q` or `Ctrl+C`.
   * Assert process exits successfully and terminal state is restored.

2. Basic profile path
   * Select Basic.
   * Confirm the module review screen appears.
   * Assert Docker, UFW, Fail2Ban, SSH hardening, and unattended upgrades are visible or selected.

3. Sandbox profile path
   * Select Sandbox.
   * Assert developer runtimes are selected.
   * Assert strict SSH/password behavior is visibly less aggressive than Basic.

4. Custom module selection
   * Select Custom.
   * Toggle Dokploy.
   * Assert Docker dependency appears selected or required.
   * Toggle conflicting reverse proxy options and assert the warning is visible.

5. Help and command palette
   * Press `?`.
   * Assert help overlay appears.
   * Press `Esc`.
   * Assert parent screen is restored.
   * Press `Ctrl+P`.
   * Type a command query.
   * Execute command.

6. Dry run
   * Create a minimal valid selection.
   * Open Run Plan.
   * Start dry run.
   * Assert visible output says no system changes were applied.

7. Responsive layout
   * Repeat startup at `80x24`, `100x32`, and `140x40`.
   * Assert key labels are visible and no required action is clipped.

## Later E2E Suite

Add these after core runtime exists:

* bracketed paste into SSH key form
* resize while modal is open
* cancellation during fake install
* failed preflight checks
* log panel scrollback
* save config and reload with `--config`
* panic path restores terminal
* mouse tests if mouse capture is enabled in a later version

---

# Test Mode Requirements

E2E tests should not touch the host machine. Toride needs an explicit test mode.

Recommended flags/env:

```text
TORIDE_E2E=1
TORIDE_NO_ANIMATION=1
TORIDE_FAKE_SYSTEM=ubuntu-24.04
TORIDE_FAKE_APPLY=1
TORIDE_CONFIG_DIR=/tmp/toride-e2e-...
```

Behavior in test mode:

* system detection returns deterministic fake data
* install effects use fake subprocesses or in-memory progress events
* animations are disabled or reduced to deterministic ticks
* network checks are mocked
* file writes go to a temp directory
* app exits cleanly on test quit command

This keeps E2E tests real at the UI boundary without making CI mutate the machine.

---

# CI Strategy

Run layers separately:

```text
cargo test --lib
cargo test --test render_snapshots
cargo test --test e2e -- --test-threads=1
```

Use Cargo's integration-test layout deliberately:

```text
tests/render_snapshots.rs
tests/e2e.rs
tests/e2e/startup.rs
tests/e2e/profiles.rs
```

`tests/e2e.rs` should declare modules from `tests/e2e/`, so `cargo test --test e2e` has one serial E2E test binary.

PTY E2E tests should run serially because they depend on timing, terminal dimensions, and subprocess cleanup. They should produce artifacts on failure:

* final terminal text buffer
* ANSI capture
* optional PNG or SVG screenshot
* app logs
* seed/config used by the test

Use fixed terminal settings in CI:

```text
TERM=xterm-256color
COLORTERM=truecolor
NO_COLOR=1 for text snapshots, unless color is being tested
```

Use timeouts everywhere:

* short visible-text waits: `1s`
* screen transition waits: `3s`
* full workflow timeout: `10s`

No E2E test should hang indefinitely.

---

# Snapshot Policy

Snapshots are useful, but they can become noisy. Use them intentionally.

Good snapshot targets:

* profile screen at `100x32`
* module review screen at `100x32`
* warning modal
* dry-run summary
* small terminal layout at `80x24`

Avoid snapshotting:

* timestamps
* random IDs
* spinner frames
* live logs unless normalized
* terminal-dependent color escapes unless the test is specifically about color

Prefer semantic assertions for workflows:

```text
assert visible "Basic Profile"
assert visible "Docker"
assert focused "Run Plan"
assert not visible "Password login remains enabled"
```

Use snapshots when the layout itself matters.

---

# Implementation Plan

## Phase 1: Make the App Testable

* Keep `update` pure.
* Keep rendering as `view(frame, &model)`.
* Add deterministic test constructors such as `Model::initial_for_test()`.
* Add `TORIDE_E2E` test mode.
* Add `--no-animation` or `TORIDE_NO_ANIMATION=1`.
* Ensure every screen has stable visible labels.

## Phase 2: Add Render Snapshots

* Add `insta`.
* Add snapshot tests for core screens.
* Run at fixed sizes.
* Normalize dynamic content.

## Phase 3: Add PTY E2E

* Start with `testty`.
* Add `tests/e2e.rs` as the E2E integration-test target.
* Add journey modules under `tests/e2e/`, starting with `tests/e2e/startup.rs`.
* Add helpers for common key flows.
* Capture screen artifacts on failure.
* Run E2E serially in CI.

## Phase 4: Expand Carefully

* Add one E2E test per important user journey.
* Keep deep rule coverage in unit tests.
* Avoid duplicating every reducer branch in E2E.

---

# Tooling Notes

Relevant references:

* Ratatui `TestBackend`: current rustdoc describes it as an in-memory backend intended for integration tests of the entire terminal UI; for lower-level widget tests, prefer direct buffer testing.
* Ratatui testing recipes: official docs point to app testing and `insta` snapshots, with a caveat that simple color snapshot assertions are not supported in that recipe as of now.
* `testty`: Rust-native PTY-driven TUI E2E framework with terminal-state assertions.
* `portable-pty`: cross-platform PTY API for opening PTYs, spawning commands through the slave side, and reading/writing through the master side.
* `expectrl`: Rust automation library for interactive terminal programs; useful for prompt-style flows, weaker for full-screen TUI layout assertions.
* `vt100`: terminal parser useful for turning ANSI output into a queryable screen buffer with cell-level state.
* Textual's testing docs are a useful comparison point: Python Textual provides a `Pilot` that presses keys, clicks, changes terminal size, pauses, and supports snapshot testing. For Ratatui we assemble the same idea with PTY tooling.

Sources:

* https://ratatui.rs/concepts/backends/
* https://ratatui.rs/recipes/testing/
* https://ratatui.rs/recipes/testing/snapshots/
* https://docs.rs/crate/testty/0.6.12
* https://docs.rs/testty/latest/testty/
* https://docs.rs/portable-pty/latest/portable_pty/
* https://docs.rs/expectrl
* https://docs.rs/vt100/latest/vt100/
* https://textual.textualize.io/guide/testing/
