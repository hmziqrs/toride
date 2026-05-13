# v0.1 Implementation Plan

Scope from `plan.md` MVP section. Follows `design.md` architecture.

---

## Phase 1: Project Scaffolding & Core Types

### 1.1 Cargo.toml dependencies
- `ratatui` v0.30+
- `crossterm` (features: `event-stream`)
- `tachyonfx`
- `tokio` (full)
- `tokio-util` (features: `rt`)
- `futures`
- `tokio-stream` (features: `io-util`)
- `clap` (derive)
- `serde` / `serde_json` / `toml`
- `color-eyre`
- `thiserror`
- `tracing` / `tracing-subscriber` (features: `env-filter`, `json`)
- `tracing-appender`
- `reqwest` (default-tls disabled, features: `rustls-tls`)
- `sha2` / `hex`
- `which`
- `nix` (features: `user`, `signal`, `fs`)
- `dirs`
- `async-trait`
- `insta` (dev)

### 1.2 Directory structure
```
src/
├─ main.rs              // CLI entry, dispatches TUI or non-interactive
├─ tui/
│  ├─ mod.rs
│  ├─ runtime.rs        // event loop, effect spawner, EffectManager
│  ├─ model.rs          // Model + initial state
│  ├─ update.rs         // pure reducer
│  ├─ effects.rs        // Effect runner (tokio tasks)
│  ├─ view.rs           // root view fn
│  ├─ theme.rs          // tokens, themes, palette
│  ├─ glyphs.rs         // unicode + ascii fallbacks
│  ├─ caps.rs           // TerminalCaps detection
│  ├─ animation.rs      // tachyonfx wiring + effect specs
│  ├─ keymap.rs         // KeyMap + binding registry
│  ├─ forms/
│  │  ├─ mod.rs
│  │  └─ validators.rs  // username, ssh_key, port, swap_size validators
│  └─ widgets/
│     ├─ mod.rs
│     ├─ header.rs
│     ├─ sidebar.rs
│     ├─ module_list.rs
│     ├─ module_card.rs
│     ├─ status_bar.rs
│     ├─ progress_panel.rs
│     ├─ log_view.rs
│     ├─ toast.rs
│     ├─ palette.rs
│     ├─ help.rs
│     ├─ confirm.rs
│     └─ splash.rs
├─ profiles/
│  ├─ mod.rs
│  ├─ basic.rs
│  └─ custom.rs
├─ modules/
│  ├─ mod.rs            // SetupModule trait, registry
│  ├─ system_update.rs
│  ├─ swap.rs
│  ├─ user_ssh.rs
│  ├─ ufw.rs
│  ├─ docker.rs
│  └─ mise.rs
├─ executor/
│  ├─ mod.rs
│  ├─ command.rs
│  ├─ plan.rs
│  ├─ dry_run.rs
│  └─ logs.rs
├─ system/
│  ├─ mod.rs
│  ├─ os_detect.rs
│  ├─ package_manager.rs
│  ├─ users.rs
│  ├─ services.rs
│  └─ ports.rs
└─ config/
   ├─ mod.rs
   └─ schema.rs
```

### 1.3 Core type definitions
- `Action` enum (design.md line 41-88)
- `Effect` enum: DetectSystem, GeneratePlan, RunInstall, CancelInstall, WriteConfig, LoadConfig, OpenUrl, Sleep, PushFx(tachyonfx spec)
- `Model` struct (design.md line 15-34)
- `Screen` enum: Welcome, ProfileSelect, ModuleSelect, Configure, Preflight, Apply, Summary + overlays (Help, Palette, Search, Confirm)
- `Profile` enum: Basic, Custom
- `ModuleId` enum: all v0.1 module IDs
- `ModuleState` struct
- `SelectionState` struct
- `Plan`, `RunState`, `ProgressEvent`, `Outcome`
- `TerminalCaps`, `Theme`, `SemanticToken`
- `FocusId` — tracks focused widget/element
- `Toast`, `LogLine`

### 1.4 TerminalCaps detection
- Read COLORTERM, NO_COLOR, FORCE_COLOR, LANG/LC_*, TERM
- Width/height from crossterm terminal size

### 1.5 Theme + Glyphs
- Dark palette colors (design.md line 244-258)
- SemanticToken → Color mapping
- Glyph set with ASCII fallbacks (design.md line 279-287)
- `Theme::style(token) -> Style`
- `Theme::glyph(g) -> &str`

### 1.6 Logging setup
- `tracing_subscriber` with `env-filter` + `json` features
- Dual output: text formatter to `/var/log/toride/setup.log`, JSON formatter to `/var/log/toride/actions.jsonl`
- `tracing_appender::rolling::RollingFileAppender` (daily rotation) + `NonBlocking` wrapper
- Non-root fallback: `~/.local/state/toride/logs/`
- Drop counter for overflow in non-blocking writer

---

## Phase 2: TUI Runtime, Reducer, View

### 2.1 Event loop (src/tui/runtime.rs)
- `color_eyre::install()` → `ratatui::init()` → bracketed paste enable
- `mpsc::unbounded_channel<Action>`
- Spawn: terminal events (crossterm EventStream), logical tick (4 Hz), signal watcher
- Main loop: recv action → update → spawn effects → conditional render
- Render: `view(frame, &model)` → `effects.process_effects()`
- AnimationTick scheduling when tachyonfx has active effects
- Quit handling: disable bracketed paste → `ratatui::restore()`

### 2.2 Update reducer (src/tui/update.rs)
- Pure `update(&mut Model, Action) -> Vec<Effect>`
- Navigation: Push/Pop/Replace screen stack
- Module selection: ToggleModule, SelectAll, SelectNone, InvertSelection, ResetProfileDefaults
- Profile: set defaults on selection
- Forms: field changes, submit, validation
- Overlays: OpenHelp, CloseOverlay, OpenPalette, PaletteInput, PaletteExec
- Results: OsDetected, PlanReady, InstallProgress, InstallDone, Error, Toast
- Set `needs_render = true` on all display-changing actions
- Set `should_quit = true` on Quit (with confirmation guard for active RunState)
- Confirmation guards for: Apply plan, disable root SSH, disable password SSH, enable UFW, change SSH port, overwrite toride.toml, reset profile defaults, run remote scripts

### 2.3 View layer (src/tui/view.rs)
- Root `view(frame, &model)` dispatches to screen views
- App layout: Header(1) / Body(flex) / StatusBar(1)
- Body: Sidebar(24) + Content when width >= 100, else single column
- Min size guard: 80x24, render "please resize" placeholder below

### 2.4 Widgets
Each widget: `render(area, frame, &Model)` — stateless, reads from Model.

- **header.rs**: App name "Toride", breadcrumb from screen_stack, host badge (OS + IP)
- **sidebar.rs**: Category tree with module counts, left-edge focus indicator
- **module_list.rs**: Virtualized checklist grouped by category, inline status icons
- **module_card.rs**: Expanded detail with description, deps, conflicts, options form
- **status_bar.rs**: Context-aware keybinding hints (left), mode chip (right)
- **progress_panel.rs**: Per-step rows with spinner/progress/log tail
- **log_view.rs**: Autoscrolling log with filter
- **toast.rs**: Bottom-right notification stack
- **palette.rs**: Command palette modal with fuzzy filter
- **help.rs**: Keybinding cheat sheet modal
- **confirm.rs**: Confirmation dialog modal with Yes/No
- **splash.rs**: Startup logo

### 2.5 Keymap (src/tui/keymap.rs)
- Binding registry: key → Action per screen context
- Global bindings from design.md keyboard map
- Navigation, module selection, form, apply screen (f=follow-tail, s=skip failed, R=retry), palette sections
- Help overlay bindings: `/` search, `j`/`k` scroll
- Apply screen: `f` follow-tail, `s` skip failed, `R` retry
- Palette commands: :plan, :apply, :dry-run, :save, :load, :reset, :theme, :log, :export, :quit
- Single source of truth for status bar hints and help overlay

### 2.6 Animation wiring (src/tui/animation.rs)
- tachyonfx EffectManager integration
- Effect catalog from design.md animation table
- Reduced motion: TORIDE_NO_ANIM=1 collapses effects to final state

### 2.7 Effect runner (src/tui/effects.rs)
- `spawn_effect(Effect, action_tx, cancel)` on tokio tasks
- DetectSystem → run OS detection → post OsDetected
- GeneratePlan → collect module plans → post PlanReady
- RunInstall → execute plan → stream InstallProgress → post InstallDone
- CancelInstall → cancel token
- WriteConfig/LoadConfig → serialize/deserialize
- OpenUrl → open URL in browser (v0.1 stub)
- Sleep → delayed action dispatch

---

## Phase 3: Module System, Profiles, Executor

### 3.1 SetupModule trait (src/modules/mod.rs)
```rust
#[async_trait]
trait SetupModule: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn dependencies(&self) -> &[ModuleId];
    fn conflicts(&self) -> &[ModuleId];
    fn category(&self) -> Category;
    async fn preflight(&self, ctx: &Context) -> Result<PreflightResult>;
    async fn plan(&self, ctx: &Context) -> Result<Vec<InstallAction>>;
    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> Result<ApplyOutcome>;
    async fn verify(&self, ctx: &Context) -> Result<VerifyResult>;
}
```
- Module registry: BTreeMap<ModuleId, Box<dyn SetupModule>>
- Dependency validation at startup
- InstallAction enum (renamed from Action to avoid collision with TUI Action)

### 3.2 InstallAction enum
```rust
enum InstallAction {
    AptInstall { packages: Vec<&'static str> },
    AptRepoAdd { name: &'static str, key_url: String, sources_line: String, sha256: String },
    WriteFile { path: PathBuf, content: String, mode: u32, backup: bool },
    AppendLine { path: PathBuf, line: String, marker: &'static str },
    Systemctl { unit: String, op: SystemctlOp },
    UfwRule { rule: String },
    UserCreate { name: String, groups: Vec<String>, shell: PathBuf },
    UserAddKey { user: String, key: String },
    DownloadScript { url: String, sha256: String, run_as: String, env: Vec<(String, String)> },
    Exec { cmd: String, args: Vec<String>, env: Vec<(String, String)>, as_user: Option<String> },
}
```
- Each variant has `to_shell_preview() -> String`

### 3.3 v0.1 Modules
- **system_update.rs**: apt update + upgrade + baseline packages (git, curl, wget, unzip, jq, etc.)
- **swap.rs**: Create swap file, validate size, swapon
- **user_ssh.rs**: Create user, add SSH key, sshd drop-in hardening, cloud-init override
- **ufw.rs**: Install UFW, default deny, allow SSH port, enable
- **docker.rs**: Docker repo + engine + compose plugin + log rotation + user group
- **mise.rs**: Install mise, configure runtimes (Node, Bun, Deno, Go, Rust, Python)

### 3.4 Executor (src/executor/)
- Sequential plan execution with progress streaming
- Each InstallAction → shell command(s)
- flock wrapper for apt operations
- Backup before file modifications
- Cancel support via CancellationToken
- Line-by-line stdout/stderr streaming via tokio::process + LinesStream

### 3.5 Profiles (src/profiles/)
- **basic.rs**: Preselected modules per plan.md Basic Profile
- **custom.rs**: Empty selection, user picks everything

### 3.6 System detection (src/system/)
- OS detect: /etc/os-release parsing
- Package manager detection (apt)
- User detection: current user, root check
- Service detection: systemd present?
- Port scanning: what's in use
- Existing tooling: docker, node, etc.

---

## Phase 4: CLI, Config, Integration

### 4.1 CLI (clap)
```
toride                        # Launch TUI (default)
toride plan --profile basic   # Generate plan, print to stdout
toride plan --json            # JSON plan output
toride apply --profile basic  # Non-interactive apply (requires root)
toride apply --config path    # Apply from config file
```
Flags:
- `--profile <basic|custom>`
- `--config <path>`
- `--user <name>`
- `--ssh-key <path>`
- `--json`
- `--no-animation`
- `--no-color` / `--color=always`

### 4.2 Config schema (src/config/schema.rs)
- TOML-based config matching plan.md example
- Serialize/deserialize with serde
- Save/load from toride.toml

### 4.3 main.rs integration
- Parse CLI args
- If no subcommand: launch TUI
- If `plan`: generate plan, print/dry-run
- If `apply`: non-interactive execution
- Root check for apply mode

---

## Phase 5: Testing

### 5.1 Unit tests (in-module #[cfg(test)])
- Reducer tests: `(Model, Action) → (Model, Vec<Effect>)`
- Profile defaults: verify basic profile selects expected modules
- Module dependency rules: Dokploy requires Docker
- Form validation: username, SSH key, port, swap size
- InstallAction::to_shell_preview output
- Plan generation from selection

### 5.2 Render snapshots (tests/render_snapshots.rs)
- `insta` + `TestBackend`
- Snapshot each screen at 100x32
- Snapshot at 80x24 for responsive check
- Normalize dynamic content (time, spinner frames)

### 5.3 E2E test harness (tests/e2e/)
- Use `testty` crate for PTY-based testing
- `tests/e2e.rs` entry point declaring modules
- `tests/e2e/startup.rs`: launch, assert profile screen, quit
- `tests/e2e/profiles.rs`: Basic + Custom profile selection flows
- `tests/e2e/custom_modules.rs`: select Custom, toggle modules, assert warnings
- `tests/e2e/help_palette.rs`: press `?`, assert help, press `Esc`, press `:`, assert palette
- `tests/e2e/dry_run.rs`: minimal selection, run plan, assert "no changes applied"
- `tests/e2e/responsive.rs`: startup at 80x24, 100x32, 140x40, assert no clipping
- Test mode: `TORIDE_E2E=1`, fake system data, no real commands

### 5.4 Test infrastructure
- `Model::initial_for_test()` constructor
- `TORIDE_E2E` test mode flag
- `TORIDE_NO_ANIMATION=1`
- `TORIDE_FAKE_SYSTEM=ubuntu-24.04`
- `TORIDE_FAKE_APPLY=1`
- `TORIDE_CONFIG_DIR=/tmp/toride-e2e-...` (temp config dir for E2E isolation)

---

## Phase 6: Audit & Commit

- Verify all v0.1 modules implemented
- Verify all screens render
- Verify all keybindings work
- Verify reducer handles all actions
- Verify executor streams progress
- Verify config save/load
- Run all tests
- Fix gaps
- Commit
