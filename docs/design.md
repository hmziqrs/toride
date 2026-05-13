# Design

Companion to `raw-plan.md`. Defines state architecture, UI design system, animation model, keyboard map, and engineering conventions. Forward references like `src/tui/...` are intended paths; the code does not yet exist.

Stack: `ratatui` v0.30+ (workspace meta-crate) + `crossterm` (`event-stream`) + `tokio` + `tachyonfx`. UI follows TEA (The Elm Architecture), the pattern Ratatui official docs recommend for scaling.

---

# State Architecture

## Pattern

Single root `Model`. All mutation flows through a pure `update(&mut Model, Action) -> Vec<Effect>`. The renderer reads from `Model` only and never mutates during draw.

```rust
struct Model {
    screen_stack: Vec<Screen>,        // back-nav stack; overlays included
    system: SystemInfo,               // OS, IP, RAM, disk, existing tooling
    profile: Profile,                 // Basic | Sandbox | Custom
    modules: BTreeMap<ModuleId, ModuleState>,
    selection: SelectionState,
    forms: HashMap<FormId, FormState>,
    plan: Option<Plan>,
    run: Option<RunState>,            // active apply
    log: RingBuffer<LogLine>,         // capped at 5000
    toasts: VecDeque<Toast>,
    palette: PaletteState,
    help: HelpState,
    theme: Theme,
    caps: TerminalCaps,               // detected at startup
    focus: FocusId,
    needs_render: bool,               // set by reducer when state may have changed
    should_quit: bool,
}
```

## Action enum (UI)

Distinct from the install-time `Action` defined in `raw-plan.md` (renamed `InstallAction` to disambiguate).

```rust
enum Action {
    // lifecycle
    Init,
    Tick,                             // 4 Hz logical tick (timers, GC)
    AnimationTick,                    // emitted only while animations are active
    Quit,

    // input (delivered by crossterm EventStream)
    Key(KeyEvent),
    Resize(u16, u16),
    FocusGained,
    FocusLost,
    Paste(String),                    // bracketed paste; used for SSH-key entry

    // navigation
    Push(Screen),
    Pop,
    Replace(Screen),

    // selection
    ToggleModule(ModuleId),
    SelectAll,
    SelectNone,
    InvertSelection,
    ResetProfileDefaults,

    // forms
    FormFieldChanged(FormId, FieldId, String),
    FormSubmit(FormId),

    // overlays
    OpenSearch,
    SearchInput(String),
    OpenPalette,
    PaletteInput(String),
    PaletteExec(PaletteCmd),
    OpenHelp,
    CloseOverlay,

    // results posted by effects
    OsDetected(SystemInfo),
    PlanReady(Plan),
    InstallProgress(ProgressEvent),
    InstallDone(Outcome),
    Error(AppError),
    Toast(Toast),
}
```

## Effects

`update` returns `Vec<Effect>` — declarative side-effect descriptions. The runtime executes them on tokio tasks and posts results back as `Action`s. The reducer never spawns tasks itself, which keeps it pure and testable.

```rust
enum Effect {
    DetectSystem,
    GeneratePlan(Selection),
    RunInstall(Plan),
    CancelInstall,
    WriteConfig(PathBuf),
    LoadConfig(PathBuf),
    OpenUrl(String),
    Sleep(Duration, Action),          // delayed action: toasts, animation chains
}
```

## Event loop (`src/tui/runtime.rs`)

Render is **event-driven**, not continuous. A naive 60 FPS loop consumes ~7% CPU at idle in release builds; toride targets ~0% idle. The render scheduler fires only when:

1. The reducer set `model.needs_render = true`, OR
2. `EffectManager::active_count() > 0` (tachyonfx has live effects), OR
3. A terminal resize occurred.

```rust
let (action_tx, mut action_rx) = mpsc::unbounded_channel::<Action>();
let cancel = CancellationToken::new();
let mut effects = tachyonfx::EffectManager::default();

spawn_terminal_events(action_tx.clone(), cancel.clone());      // EventStream → Action::Key/Resize/Paste/Focus
spawn_logical_tick(action_tx.clone(), cancel.clone(), 4.0);    // 4 Hz Action::Tick

color_eyre::install()?;                                        // pretty error reports
let mut terminal = ratatui::init();                            // raw mode + alt screen + panic restore
crossterm::execute!(stdout(), crossterm::event::EnableBracketedPaste)?;
// (mouse capture intentionally left disabled in v0.1)

spawn_signal_watcher(action_tx.clone());                       // SIGINT/SIGTERM → Action::Quit

let mut model = Model::initial(detect_caps());
let mut last_frame = Instant::now();

loop {
    let Some(action) = action_rx.recv().await else { break };
    let prev_needs = model.needs_render;
    let new_effects = update(&mut model, action);
    for eff in new_effects { spawn_effect(eff, action_tx.clone(), cancel.clone()); }

    let active = effects.active_count() > 0;
    if model.needs_render || active {
        let elapsed = last_frame.elapsed();
        last_frame = Instant::now();
        terminal.draw(|frame| {
            view(frame, &model);
            effects.process_effects(elapsed.into(), frame.buffer_mut(), frame.area());
        })?;
        model.needs_render = false;
    }

    if active && !prev_needs {
        // schedule a continuation tick at ~60 FPS while animations live
        let tx = action_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(16)).await;
            let _ = tx.send(Action::AnimationTick);
        });
    }

    if model.should_quit { break; }
}

crossterm::execute!(stdout(), crossterm::event::DisableBracketedPaste).ok();
ratatui::restore();
```

This collapses to zero CPU at idle, scales to 60 FPS during animations, and uses `ratatui::init()`'s built-in panic-hook + `color-eyre` for terminal-safe error reporting. No homegrown panic hook needed.

## Screen stack

`screen_stack: Vec<Screen>` with `Push` / `Pop` / `Replace`. `Esc` always pops one level. Overlays (Help, Palette, Search) are screens with `Screen::overlay() == true` — drawn on top of the previous frame's contents without unmounting the parent.

## Background tasks

Long-running work (apt install, downloads) runs on tokio tasks holding `action_tx`, emitting `Action::InstallProgress(...)`. They check `cancel.is_cancelled()` between awaits. `Action::CancelInstall` triggers `cancel.cancel()`; the runtime allocates a fresh token for the next run.

Subprocess output streams line-by-line:

```rust
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_stream::{wrappers::LinesStream, StreamExt};

let mut child = Command::new("apt-get")
    .args(["install", "-y", "docker-ce"])
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;

let out = LinesStream::new(BufReader::new(child.stdout.take().unwrap()).lines());
let err = LinesStream::new(BufReader::new(child.stderr.take().unwrap()).lines());
let mut merged = out.merge(err);
while let Some(Ok(line)) = merged.next().await {
    action_tx.send(Action::InstallProgress(ProgressEvent::StepLog { line, .. }))?;
    if cancel.is_cancelled() { child.kill().await?; break; }
}
```

## Persistence

* In-memory by default.
* `Ctrl+S` → `Effect::WriteConfig` serializes `Model::selection` + `Model::forms` to `toride.toml`.
* `--config path.toml` on startup hydrates state and skips the Profile screen.

---

# UI Design System

## Terminal capability detection

Run once at startup, store in `Model::caps`:

```rust
struct TerminalCaps {
    truecolor: bool,        // COLORTERM=truecolor|24bit
    unicode: bool,          // LANG/LC_* contains UTF-8 and TERM != "linux"
    no_color: bool,         // NO_COLOR set to any non-empty, non-"0" value
    force_color: bool,      // FORCE_COLOR set; overrides no_color if both somehow set
    width: u16,
    height: u16,
}
```

Theme and glyph systems read from `caps`; widgets do not branch on env vars directly.

## Theme

Two built-in themes: `dark` (default), `light`. Custom themes under `~/.config/toride/theme.toml`. All color access goes through semantic tokens — never raw `Color` in components.

```rust
enum SemanticToken {
    BgBase, BgRaised, BgOverlay,
    FgPrimary, FgSecondary, FgMuted, FgInverse,
    Accent, AccentDim,
    Success, Warning, Danger, Info,
    Border, BorderFocus,
    SelectionBg, SelectionFg,
    SpinnerActive, ProgressFill, ProgressTrack,
}
```

Default dark palette (`src/tui/theme.rs`):

```
BgBase       #0b0e14   (near-black, avoids pure #000000 halation)
BgRaised     #11151c
BgOverlay    #161b22
FgPrimary    #e6edf3
FgSecondary  #b1bac4
FgMuted      #6e7681
Accent       #7aa2f7
AccentDim    #3d4a6b
Success      #9ece6a
Warning      #e0af68
Danger       #f7768e
Info         #7dcfff
Border       #30363d
BorderFocus  #7aa2f7
```

Palette rules (per WCAG dark-mode guidance):

* Background is near-black, never `#000000` (halation, eye strain).
* Accent and status colors are desaturated — saturated colors on dark backgrounds fail WCAG AA 4.5:1 for body text.
* All `Fg* / Bg*` pairs verified ≥ 4.5:1 against `BgBase` and `BgRaised`.
* CI snapshot test computes contrast for every `(fg, bg)` token pair and fails on regression. WCAG 2 ratios for v0.1; revisit APCA when WCAG 3 stabilizes.

Color env vars:

* `NO_COLOR=<non-empty, non-zero>` — disables theming (white-on-default).
* `FORCE_COLOR=<non-empty>` — forces theming even when `NO_COLOR` set or output is piped.
* `--no-color` / `--color=always` CLI flags override both.

24-bit color when `caps.truecolor`; otherwise downgrade to ANSI 256 via crossterm's color conversion at theme resolution time.

## Glyphs

Unicode budget (ASCII fallback when `caps.unicode == false`):

```
Borders     ┌ ┐ └ ┘ ─ │ ╭ ╮ ╯ ╰
Selection   ●  ○  ☑  ☐
Status      ✓ ✗ ⚠ ⋯
Arrows      › ‹ ↑ ↓
Spinner     ⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏
Bars        ▏ ▎ ▍ ▌ ▋ ▊ ▉ █
Sparkline   ▁ ▂ ▃ ▄ ▅ ▆ ▇ █
```

`Theme::glyph(g)` returns Unicode or ASCII fallback based on `caps.unicode`.

## Layout

* App: 3 rows — Header (1) / Body (flex) / StatusBar (1).
* Body: 2 columns on width ≥ 100 (Sidebar 24 cols + Content); single column otherwise.
* Minimum terminal: 80×24. Below that, render a centered "please resize" placeholder.
* Padding via `Spacing { Xs=1, Sm=1, Md=2, Lg=3 }`.
* Use `Layout::try_areas` (Ratatui v0.30+) for compile-time constraint counts.

## Components (`src/tui/widgets/`)

* `header.rs` — app name, breadcrumb, host badge (OS + IP), clock
* `sidebar.rs` — category tree with counts, left-edge focus indicator
* `module_list.rs` — virtualized checklist, grouped, inline status icons
* `module_card.rs` — expanded detail with description, deps, conflicts, options form
* `status_bar.rs` — context-aware keybinding hints (left), mode chip (right)
* `progress_panel.rs` — per-step rows: spinner / progress / log tail
* `log_view.rs` — autoscrolling log with filter
* `toast.rs` — bottom-right notification stack
* `palette.rs` — command palette modal
* `help.rs` — keybinding cheat sheet modal
* `confirm.rs` — confirmation dialog modal
* `splash.rs` — startup logo

Each component: pure `render(area, frame, &Model)`. No state lives inside components.

### Status bar hint selection

The status bar shows up to 4 most-relevant bindings, picked by a fixed priority list per screen + `caps.width`. The hint registry is the same `keymap.rs` that powers the help overlay — single source of truth.

---

# Animations via `tachyonfx`

Use `tachyonfx` for all visual effects. It is the official Ratatui effects library and operates as a **post-render cell transform** — effects modify already-rendered buffer cells (color, character, visibility), composed via `fx::sequence` / `fx::parallel`. Do not hand-roll an animation engine.

## Runtime

```rust
let mut effects = tachyonfx::EffectManager::default();

terminal.draw(|frame| {
    view(frame, &model);                                       // render widgets normally
    effects.process_effects(elapsed.into(), frame.buffer_mut(), frame.area());
})?;
```

Effects are enqueued via `effects.add_effect(...)` from the reducer (returned as an `Effect::PushFx(spec)` and applied by the runtime). Unique effects (same identifier) cancel-and-replace, useful for focus indicators that move while the previous transition is still playing.

## Catalog

Mapping of UI events to `tachyonfx` effect specs. Durations in milliseconds. Easing names below are conceptual — at implementation time, pick the closest variant from the current `tachyonfx::Interpolation` enum. Do not hand-roll custom curves; if no built-in maps cleanly, file a request upstream rather than forking.

| Id                       | Trigger                  | Effect spec                                                                                              |
|--------------------------|--------------------------|----------------------------------------------------------------------------------------------------------|
| `splash.fade_in`         | App start                | `fx::sequence(&[fx::fade_from(BgBase, FgMuted, 250), fx::fade_to_fg(FgPrimary, 350, EaseOutCubic)])`      |
| `splash.fade_out`        | Splash dismiss           | `fx::fade_to(BgBase, BgBase, 250, EaseInOutCubic)`                                                       |
| `screen.slide_in`        | `Push(screen)`           | `fx::translate((40, 0), (0, 0), 220, EaseOutCubic)` scoped to body area                                  |
| `screen.slide_out`       | `Pop`                    | `fx::translate((0, 0), (40, 0), 180, EaseInOutCubic)`                                                    |
| `list.focus_indicator`   | Focus move               | `fx::fade_to_fg(Accent, 120, EaseOutCubic)` on new row + `fade_to_fg(FgPrimary, 120)` on old row (unique id `"focus.<list>"`) |
| `checkbox.toggle`        | `ToggleModule`           | `fx::sequence(&[fx::fade_to_fg(Success, 80), fx::fade_to_fg(FgPrimary, 60)])`                            |
| `card.expand`            | Enter on module          | `fx::coalesce(180, EaseOutCubic)` over card area                                                         |
| `card.collapse`          | Esc from module          | `fx::dissolve(140, EaseInOutCubic)`                                                                      |
| `spinner.rotate`         | Step `Running`           | Driven by render-tick frame index `(elapsed_ms / 80) % 8` — no tachyonfx effect needed                   |
| `progress.fill`          | Progress update          | Direct interpolated render in `progress_panel.rs` using block fractionals                                |
| `progress.success_pulse` | Step succeeds            | `fx::sequence(&[fx::fade_to_fg(Success, 80), fx::fade_to_fg(FgPrimary, 520, EaseOutCubic)])` on row      |
| `progress.shake`         | Step fails               | `fx::translate((0,0), (1,0), 60)` then back, 5 cycles damped — wrapped in `fx::repeat_count(...)`        |
| `toast.slide_up`         | Toast enqueue            | `fx::translate((0, 1), (0, 0), 180, EaseOutCubic)`                                                       |
| `toast.slide_down`       | Toast dismiss            | `fx::translate((0, 0), (0, 1), 140, EaseInOutCubic)` then unmount                                        |
| `palette.scale_in`       | Open palette             | `fx::coalesce(160, EaseOutBack)`                                                                         |
| `help.fade_in`           | `?` pressed              | `fx::fade_from(BgOverlay, FgPrimary, 120, EaseOutCubic)`                                                 |
| `tab.underline_slide`    | Tab change               | `fx::translate(...)` of underline glyph                                                                  |
| `search.cursor_blink`    | Search input focused     | Direct render — toggle cursor cell every 500ms based on `AnimationTick`                                  |

Effects with deterministic per-frame content (spinner, progress bar, blinking cursor) are rendered directly by widgets reading `elapsed_ms`. Effects that modify post-render cells (fades, slides, dissolves, pulses) go through `tachyonfx`.

## Reduced motion

`TORIDE_NO_ANIM=1` and `--no-animations` set `model.reduced_motion = true`. The effect dispatcher then either:

* Replaces effects with their final-state apply (instant), or
* Skips `tachyonfx` enqueue entirely for purely cosmetic effects (pulses, slides).

Functional state changes (checkbox toggle, list focus) still occur — only the transition is collapsed.

---

# Keyboard Map

Conventions:

* All bindings work on every screen unless listed as screen-local.
* `Esc` pops one level (close overlay → exit search → back one screen).
* Vim and arrow keys are aliases everywhere — matches k9s / lazygit / helix convention.
* No multi-key chords in v0.1. Namespace `g g`, `g e`, ... reserved.
* The keybinding registry (`src/tui/keymap.rs`) is the single source of truth; status bar and help overlay read from it.

## Global

| Key            | Action                                |
|----------------|---------------------------------------|
| `q`            | Quit (confirm if `RunState::Active`)  |
| `Ctrl+C`       | Cancel current op / quit              |
| `?` / `F1`     | Toggle help overlay                   |
| `:`            | Open command palette                  |
| `/`            | Open search (when list focused)       |
| `Esc`          | Pop screen / close overlay            |
| `Tab`          | Next pane                             |
| `Shift+Tab`    | Previous pane                         |
| `Ctrl+S`       | Save selection to `toride.toml`       |
| `Ctrl+L`       | Toggle log panel                      |
| `Ctrl+T`       | Dismiss top toast                     |
| `Ctrl+R`       | Reload config from disk               |
| `F2`           | Cycle theme                           |

## Navigation (lists, trees)

| Key                | Action                  |
|--------------------|-------------------------|
| `j` / `↓`          | Next item               |
| `k` / `↑`          | Previous item           |
| `h` / `←`          | Collapse / parent       |
| `l` / `→` / `Enter`| Expand / drill in       |
| `g g`              | First item (reserved)   |
| `G`                | Last item               |
| `Ctrl+D`           | Half page down          |
| `Ctrl+U`           | Half page up            |
| `PageDown`         | Page down               |
| `PageUp`           | Page up                 |
| `Home` / `End`     | First / last item       |

## Module selection (screen-local)

| Key       | Action                                    |
|-----------|-------------------------------------------|
| `Space`   | Toggle module                             |
| `Enter`   | Expand module card / open configuration   |
| `a`       | Select all visible                        |
| `n`       | Select none                               |
| `i`       | Invert selection                          |
| `r`       | Reset to profile defaults                 |
| `c`       | Toggle category collapsed                 |
| `p`       | Preview plan                              |
| `d`       | Toggle dry-run mode                       |
| `x`       | Proceed to preflight                      |

## Forms

| Key                  | Action                       |
|----------------------|------------------------------|
| `Tab` / `Shift+Tab`  | Next / previous field        |
| `Enter`              | Submit (last field) / next   |
| `Esc`                | Cancel and revert            |
| `Ctrl+W`             | Delete previous word         |
| `Ctrl+U`             | Clear field                  |
| `Ctrl+V` / bracketed paste | Paste (used for SSH key entry) |
| Standard editing     | Arrow keys, Home, End, etc.  |

## Apply screen

| Key       | Action                                 |
|-----------|----------------------------------------|
| `j` / `k` | Focus next / previous step row         |
| `Enter`   | Expand step log                        |
| `f`       | Toggle follow-tail                     |
| `Ctrl+C`  | Cancel running plan (confirm)          |
| `s`       | Skip current step (only if `Failed`)   |
| `R`       | Retry current step                     |

## Palette

| Key       | Action                       |
|-----------|------------------------------|
| Type      | Fuzzy filter commands        |
| `↑` / `↓` | Navigate matches             |
| `Enter`   | Execute selected command     |
| `Esc`     | Close palette                |

Initial commands:

```
:plan                    Preview plan
:apply                   Run apply
:dry-run                 Run in dry-run mode
:save <path>             Save config
:load <path>             Load config
:reset                   Reset to profile defaults
:theme dark|light|<name> Switch theme
:log                     Toggle log panel
:export json|toml <path> Export plan
:quit                    Quit
```

## Help overlay

| Key                | Action               |
|--------------------|----------------------|
| `?` / `Esc` / `q`  | Close overlay        |
| `/`                | Search bindings      |
| `j` / `k`          | Scroll               |

---

# Best Practices

## Terminal init and panic safety

Use `ratatui::init()` (v0.30+) — enters raw mode, switches to the alternate screen, and installs a panic hook that restores the terminal on panic. Pair with `color_eyre::install()` for pretty error reports. Do not roll your own panic hook.

```rust
color_eyre::install()?;
let mut terminal = ratatui::init();
crossterm::execute!(stdout(), crossterm::event::EnableBracketedPaste)?;
// ... run loop ...
crossterm::execute!(stdout(), crossterm::event::DisableBracketedPaste).ok();
ratatui::restore();
```

Mouse capture stays off in v0.1 (avoids text-selection conflicts in many terminals). Bracketed paste is on so SSH-key paste lands as a single `Action::Paste(String)` rather than character-by-character key events.

## Signal handling

```rust
fn spawn_signal_watcher(action_tx: UnboundedSender<Action>) {
    use tokio::signal::{ctrl_c, unix::{signal, SignalKind}};
    tokio::spawn(async move {
        let mut term = signal(SignalKind::terminate()).unwrap();
        tokio::select! {
            _ = ctrl_c() => {}
            _ = term.recv() => {}
        }
        let _ = action_tx.send(Action::Quit);
    });
}
```

`Action::Quit` flows through the reducer like any other action so confirmation guards (e.g. while a run is active) apply uniformly. If a second signal arrives, abort immediately — second-press = "I mean it".

## Confirmation dialogs

Destructive or hard-to-reverse actions go through a confirmation modal screen pushed onto the stack. The modal is a `Screen::Confirm(ConfirmSpec)` overlay with explicit Yes/No buttons whose labels restate the action ("Disable password login" — not "OK") per modal best practice. No default selection.

Actions requiring confirmation:

* Quit while `RunState::Active`
* Cancel a running plan (`Ctrl+C` during Apply)
* Disable root SSH login
* Disable password SSH login
* Apply with unsafe-combination warnings unresolved
* Overwrite an existing `toride.toml`
* Reset profile defaults (discards user-made changes)

Modal implementation: a single `widgets::confirm.rs` (added to the components list) renders the spec; no external crate. The community crates `tui-confirm-dialog` / `tui-dialog` are options if scope grows.

## Screen states

Every screen handles four states, not just the happy path:

* **Loading** — show shimmer/spinner in the data region (no full-screen overlay).
* **Empty** — explain why nothing's here and the next action ("No modules match `xyz` — `Esc` to clear search").
* **Error** — show the error inline with a `Retry` action; persistent errors also toast.
* **Ready** — the normal render.

This applies to the Welcome screen (system detection in flight), Module Selection (waiting for registry validation), Plan Preview (plan generation), Apply (active run).

## Form validation

Forms validate per-field on blur and on submit. Each `FormField` has a `validator: fn(&str) -> Result<(), String>`. The field renders with a red border and inline message when invalid. Submit is disabled while any field is invalid; the status bar shows the first invalid field's message.

Common validators (forward references in `src/tui/forms/validators.rs`):

* `username` — POSIX-portable name regex, not in `/etc/passwd`
* `ssh_public_key` — parses as OpenSSH authorized_keys line
* `path_exists` / `path_writable`
* `swap_size` — `<integer>(K|M|G)`, within free disk
* `port` — 1–65535, not currently bound

## Error handling

* `color_eyre::Result<T>` at the app boundary; `color-eyre` installed via `init_with_options`.
* `thiserror`-derived domain errors at module boundaries (`ModuleError`, `IoError`, `NetworkError`).
* No `unwrap()` outside `main` and tests. `expect()` only with an `// invariant:` comment explaining why.
* `Action::Error(AppError)` surfaces non-fatal errors as toasts; the run continues.

## Async hygiene

* Never hold `Mutex` / `RwLock` across `.await`. Use message passing.
* Every background task owns a `CancellationToken` clone and checks it between awaits.
* `mpsc::unbounded_channel` for action dispatch. Bounded channels only for subprocess byte streams where backpressure matters.
* Subprocess streaming: `tokio::process::Command` + `Stdio::piped()` + `LinesStream::new(BufReader::new(...).lines())` + `StreamExt::merge` to combine stdout and stderr.

## Module conventions

* One module = one file under `src/modules/`.
* Each module exports a zero-sized `pub struct ModuleX;` implementing `SetupModule`.
* Ids are kebab-case: `"ssh-hardening"`, `"docker"`, `"mise"`.
* Dependencies validated against the registry at startup; missing-id is a startup error.

## Testing

* `update()` is pure: golden-file tests with `(Model, Action) → (Model, Vec<Effect>)`.
* Effects mocked: `Effect::RunInstall(plan)` records the plan in tests instead of executing.
* Layout regressions: `insta` snapshots of `TestBackend` frame buffers.
* Contrast: CI computes WCAG ratios for all `(fg, bg)` token pairs and fails on regression.
* Module subprocess tests use a `Sandbox` trait selecting `RealExec` or `FakeExec` at construction.

## Rendering rules

* Render is event-driven. Reducer sets `model.needs_render = true` on state changes that affect display. Animations gate continuation ticks.
* Do not pre-cache layout — Ratatui's frame diff already minimizes terminal writes. Caching is an anti-pattern here.
* No allocation in `view()` hot path beyond what ratatui itself does.
* All text rendered via `Theme::style(token)` — never inline `Color::Rgb(...)` in components.
* Widgets must be deterministic functions of `(area, &Model)`.

## Logging

* `tracing` with `tracing-subscriber` (`env-filter`, `json`).
* File output via `tracing_appender::rolling::RollingFileAppender` with `Rotation::DAILY`, wrapped in `tracing_appender::non_blocking::NonBlocking` so disk I/O never blocks the reducer.
* Two log files:
  * `/var/log/toride/setup.log` — human-readable (text formatter)
  * `/var/log/toride/actions.jsonl` — structured (JSON formatter)
* If the non-blocking writer's queue overflows, drop and increment a `dropped_lines` counter shown in the status bar.
* UI log buffer is a separate `RingBuffer<LogLine>` capped at 5000, populated by `Action::InstallProgress` — independent of disk logging.

## Accessibility & degradation

* `TORIDE_NO_ANIM=1` / `--no-animations` collapses all animations to final state.
* `NO_COLOR=<non-empty>` / `--no-color` disables theming. `FORCE_COLOR=<non-empty>` overrides.
* ASCII glyph fallbacks when `caps.unicode == false`.
* WCAG 2 AA contrast (4.5:1) for body text in default themes; CI-verified. Plan to revisit with APCA once WCAG 3 stabilizes.

## Performance budgets

* Cold start to first paint: < 80ms.
* Reducer per action: p99 < 5ms on a 4-core VPS. (Render cost dominated by terminal write syscalls — Ratatui's own diff/draw is sub-ms.)
* Idle CPU: ~0%. Active-animation CPU: < 10% single-core in release builds.
* Render frame budget while animating: 16ms (60 FPS). Exceeding logs a warning to the trace stream.

## Code structure (forward reference)

```
src/tui/
├─ runtime.rs        // event loop, effect spawner, EffectManager
├─ model.rs          // Model + initial state
├─ update.rs         // pure reducer
├─ effects.rs        // Effect runner (tokio tasks)
├─ view.rs           // root view fn
├─ theme.rs          // tokens, themes, palette, contrast tests
├─ glyphs.rs         // unicode + ascii fallbacks
├─ caps.rs           // TerminalCaps detection
├─ animation.rs      // tachyonfx wiring + effect specs
├─ keymap.rs         // KeyMap + binding registry
└─ widgets/
   ├─ header.rs
   ├─ sidebar.rs
   ├─ module_list.rs
   ├─ module_card.rs
   ├─ status_bar.rs
   ├─ progress_panel.rs
   ├─ log_view.rs
   ├─ toast.rs
   ├─ palette.rs
   ├─ help.rs
   ├─ confirm.rs
   └─ splash.rs
```

## Ratatui v0.30 notes

* Depend on the `ratatui` meta-crate (workspace split is transparent to users).
* Use `Layout::try_areas` for compile-time constraint-count checks.
* `List::highlight_symbol` takes `Into<Line>` — pass styled `Line` directly.
* `Text`, `Line`, `Span` `patch_style`/`reset_style` are owning (take self, return self).

## Conventions for future contributors

* New screen: extend `Screen`, add `view_<screen>`, register keybindings in `keymap.rs`, add at least one snapshot test.
* New module: file under `src/modules/`, register in `modules/mod.rs`, declare deps/conflicts, write a `to_shell_preview` test per `Action` variant emitted.
* New animation: add to the catalog table here using `tachyonfx` primitives only. No ad-hoc easing curves.
* New keybinding: register in `keymap.rs`, add to relevant section here; the help overlay reads from the registry.
