# Rust Library Plan: Typed Mise Wrapper for Dynamic Language/Tool Version Management

## 0. Product Goal

Build a Rust library/crate that lets other Rust applications manage developer tools and language runtimes through `mise`.

This is not a CLI app.

This is not a reimplementation of `nvm`, `gvm`, `pyenv`, `rustup`, `rbenv`, `asdf`, or mise.

This is a typed Rust integration layer over mise that gives Rust apps a clean API for:

* discovering supported tools
* listing remote versions
* installing versions
* uninstalling versions
* setting project/global versions
* reading current active versions
* resolving executable paths
* generating runtime environments
* running commands inside a tool environment
* checking missing/outdated/prunable tools
* managing lockfiles
* managing mise config safely
* doing diagnostics
* supporting many languages dynamically

The core philosophy:

> Use mise as the proven engine. Use Rust to provide a safe, typed, ergonomic library API around it.

## 1. Why Mise Is the Right Engine

Mise already solves the hard parts:

* language version management
* tool installation
* version discovery
* project-local config
* global config
* environment activation
* shims / PATH management
* lockfile support
* backend abstraction
* core language plugins
* third-party backends
* registry aliases
* plugin support
* install cache
* pruning unused versions
* detecting outdated versions
* running commands inside a selected runtime

This means our crate should avoid home-cooked installers.

Bad approach:

```text
our crate downloads Node itself
our crate installs Go itself
our crate compiles Ruby itself
our crate shells into nvm/gvm/pyenv manually
our crate parses every language’s upstream release feed
```

Good approach:

```text
our crate asks mise what versions exist
our crate asks mise to install the requested version
our crate reads mise JSON output
our crate writes/edits mise.toml safely
our crate exposes a typed Rust API
```

## 2. Hard Decision: Wrapper, Not Vendored Internal API

### 2.1 Do not vendor mise internals

Vendoring mise source directly sounds attractive because mise is Rust, but it is the wrong default.

Problems:

* mise is primarily an application, not a stable public library API
* internal modules can change without semver guarantees for external users
* you inherit mise’s full dependency graph
* you need to keep rebasing patches
* private assumptions will break between releases
* async/runtime/globals/config paths may not be designed for embedding
* your crate becomes tightly coupled to mise internals
* upstream security fixes require painful merges
* build times and binary size may explode
* your public API accidentally mirrors mise internals

### 2.2 Use the mise binary as an engine

This is the better architecture:

```text
Rust app
  ↓
our typed crate
  ↓
mise command adapter
  ↓
mise binary
  ↓
mise backends / registry / installs / config
```

This is not “raw dogging commands” if done properly.

We will not use `std::process::Command` directly everywhere.

We will use a proven command execution crate, preferably:

* `duct` for simple, robust command execution
* or `tokio::process` only behind an async feature
* `which` for binary discovery
* `serde_json` for JSON output
* `camino` for UTF-8 paths
* `fs-err` for better filesystem errors
* `tempfile` for tests
* `thiserror` or `miette` for errors
* `tracing` for observability

The wrapper must be centralized in one adapter layer so the rest of the crate never manually constructs shell commands.

## 3. Crate Name Ideas

Good names:

* `mise-manager`
* `mise-control`
* `mise-sdk`
* `mise-rs-sdk`
* `mise-wrapper`
* `devtool-manager`
* `runtime-manager`
* `toolchain-manager`
* `toolenv`

Best practical name:

```text
mise-manager
```

Reason:

* clear
* boring
* exact
* library-oriented
* not overbranded
* easy to understand on crates.io

## 4. Target Users

This crate is for Rust applications that need to manage toolchains dynamically.

Examples:

* IDEs
* desktop apps
* dev environment managers
* coding agent platforms
* server automation tools
* project bootstrap tools
* CI preparation tools
* local development dashboards
* language runtime selectors
* monorepo tooling
* self-hosted deployment helpers

Not for:

* replacing mise CLI
* writing another package manager
* managing system packages like apt/pacman/brew
* installing OS libraries required to compile Ruby/Python
* being a full shell activation framework

## 5. High-Level Capabilities

The crate should expose these major capability areas:

```text
Mise discovery
Mise installation/bootstrap
Tool registry
Remote version listing
Installed version listing
Active version resolution
Tool install
Tool uninstall
Tool use / pin / unuse
Environment generation
Command execution inside environments
Path resolution
Outdated checks
Upgrade planning
Pruning
Cache clearing
Lockfile management
Config read/write
Settings read/write
Diagnostics
Language-specific convenience helpers
```

## 6. Core API Shape

### 6.1 Main client

```rust
pub struct Mise {
    binary: MiseBinary,
    cwd: Option<Utf8PathBuf>,
    env: BTreeMap<String, String>,
    mode: MiseMode,
}
```

### 6.2 Builder

```rust
let mise = Mise::builder()
    .binary("mise")
    .cwd("/path/to/project")
    .no_config(false)
    .no_env(false)
    .locked(false)
    .build()?;
```

### 6.3 Sync-first design

Start sync-first.

Most server/admin crates are easier to consume with blocking APIs.

```rust
mise.install("node@22")?;
mise.use_tool(UseRequest::global("node@22"))?;
let env = mise.env_json(["node@22"])?;
```

Optional later:

```rust
features = ["async"]
```

Use `tokio::process` only behind that feature.

## 7. Command Adapter Layer

All command execution must go through one internal module:

```text
src/command/
  mod.rs
  runner.rs
  output.rs
  errors.rs
```

### 7.1 Runner trait

```rust
pub trait CommandRunner {
    fn run(&self, cmd: MiseCommand) -> Result<CommandOutput, MiseError>;
}
```

### 7.2 Real runner

Uses `duct`.

Responsibilities:

* set cwd
* set env
* pass args without shell interpolation
* capture stdout
* capture stderr
* capture exit code
* support timeouts
* redact secrets in logs
* normalize UTF-8 output
* parse JSON only at API boundary
* include command context in errors

### 7.3 Fake runner for tests

Allows snapshot/unit tests without real mise installed.

```rust
pub struct FakeRunner {
    responses: Vec<FakeResponse>,
}
```

This is critical because we do not want every unit test to download tools.

## 8. Error Model

Create structured errors.

```rust
pub enum MiseError {
    BinaryNotFound,
    UnsupportedVersion { version_output: String },
    CommandFailed {
        command: String,
        exit_code: Option<i32>,
        stdout: String,
        stderr: String,
    },
    JsonParse {
        command: String,
        source: serde_json::Error,
        stdout: String,
    },
    Io(std::io::Error),
    Config(ConfigError),
    Timeout,
}
```

Expose clean user-facing errors:

```rust
pub enum ToolInstallError {
    ToolNotFound,
    VersionNotFound,
    NetworkFailed,
    ChecksumFailed,
    DependencyMissing,
    PermissionDenied,
    MiseFailed(MiseError),
}
```

Do not collapse everything into strings.

## 9. Main Modules

Recommended structure:

```text
src/
  lib.rs

  client.rs
  builder.rs
  error.rs

  command/
    mod.rs
    runner.rs
    real.rs
    fake.rs
    output.rs

  binary/
    mod.rs
    discovery.rs
    install.rs
    version.rs

  tool/
    mod.rs
    spec.rs
    registry.rs
    remote.rs
    installed.rs
    active.rs
    install.rs
    uninstall.rs
    upgrade.rs
    prune.rs

  config/
    mod.rs
    model.rs
    read.rs
    write.rs
    merge.rs
    path.rs

  env/
    mod.rs
    generated.rs
    activation.rs
    shell.rs

  exec/
    mod.rs
    request.rs
    result.rs

  lockfile/
    mod.rs
    model.rs
    update.rs
    verify.rs

  diagnostics/
    mod.rs
    doctor.rs
    path.rs
    checks.rs

  languages/
    mod.rs
    node.rs
    bun.rs
    deno.rs
    go.rs
    python.rs
    rust.rs
    ruby.rs
    java.rs
    generic.rs

  serde/
    mod.rs
    json_outputs.rs

  tests/
```

## 10. Tool Spec Model

The entire crate should revolve around a normalized tool spec.

```rust
pub struct ToolSpec {
    pub backend: Option<String>,
    pub name: String,
    pub version: Option<VersionRequest>,
    pub options: BTreeMap<String, toml::Value>,
}
```

Examples:

```text
node@22
bun@latest
deno@2
go@1.23
python@3.12
rust@stable
ruby@3.3
java@temurin-21
npm:prettier@latest
cargo:ripgrep@latest
go:github.com/jesseduffield/lazygit@latest
ubi:BurntSushi/ripgrep[exe=rg]
```

### 10.1 VersionRequest

```rust
pub enum VersionRequest {
    Latest,
    Prefix(String),
    Exact(String),
    Alias(String),
    Channel(String),
}
```

Do not force everything into `semver::Version`.

Many tool versions are not pure semver:

* `lts`
* `latest`
* `stable`
* `beta`
* `temurin-21`
* `prefix:1.20`
* `anaconda`
* `3.12t`
* `graalvm-21`
* `nightly`
* `1.82`

## 11. Registry Support

Expose registry APIs:

```rust
mise.registry() -> Result<Vec<RegistryTool>>
mise.registry_tool("poetry") -> Result<RegistryTool>
mise.registry_by_backend("aqua") -> Result<Vec<RegistryTool>>
```

Model:

```rust
pub struct RegistryTool {
    pub name: String,
    pub full_name: String,
    pub backend: Option<String>,
    pub security: Option<RegistrySecurity>,
}
```

Use `mise registry --json`.

Support `--security` as optional because it can be slower.

## 12. Version Discovery

Expose:

```rust
mise.list_remote("node")?;
mise.list_remote_prefix("node", "22")?;
mise.list_remote_json("github:cli/cli")?;
```

Request model:

```rust
pub struct ListRemoteRequest {
    pub tool: ToolName,
    pub prefix: Option<String>,
    pub all: bool,
    pub prerelease: bool,
    pub minimum_release_age: Option<String>,
    pub strict_metadata: bool,
    pub no_versions_host: bool,
}
```

Important features to support:

* JSON output
* prereleases
* minimum release age
* strict metadata
* prefix search
* no versions host
* cache caveat
* all tools

Use this for dynamic UI dropdowns.

## 13. Installed / Active Tool Listing

Expose:

```rust
mise.list()?;
mise.list_installed()?;
mise.list_current()?;
mise.list_missing()?;
mise.list_prunable()?;
mise.list_outdated()?;
```

Request model:

```rust
pub struct ListToolsRequest {
    pub installed_only: bool,
    pub current_only: bool,
    pub global_only: bool,
    pub local_only: bool,
    pub missing_only: bool,
    pub prunable_only: bool,
    pub outdated: bool,
    pub all_sources: bool,
    pub prefix: Option<String>,
}
```

Use `mise ls --json`.

Return:

```rust
pub struct ToolStatus {
    pub tool: String,
    pub version: String,
    pub requested: Option<String>,
    pub installed: bool,
    pub active: bool,
    pub missing: bool,
    pub source: Option<Utf8PathBuf>,
    pub outdated: Option<bool>,
}
```

## 14. Install API

Expose:

```rust
mise.install(["node@22", "python@3.12"])?;
```

Request:

```rust
pub struct InstallRequest {
    pub tools: Vec<ToolSpec>,
    pub jobs: Option<usize>,
    pub force: bool,
    pub raw: bool,
    pub locked: bool,
    pub yes: bool,
}
```

Support:

* install all tools from current config
* install selected tools
* install with jobs
* install in locked mode
* install with force
* install with current working directory

Important behavior:

* `mise install` installs tools from config if no tool is supplied.
* `mise use` also installs if the tool is not already installed.
* The crate should expose both behaviors separately.

## 15. Uninstall API

Expose:

```rust
mise.uninstall(["node@20.0.0"])?;
```

Request:

```rust
pub struct UninstallRequest {
    pub tools: Vec<ToolSpec>,
    pub dry_run: bool,
}
```

This removes installed tool versions.

It should not necessarily remove references from `mise.toml`.

That is a different operation: `unuse`.

## 16. Use / Pin / Unuse API

### 16.1 Use

Expose:

```rust
mise.use_tool(UseRequest::local("node@22"))?;
mise.use_tool(UseRequest::global("python@3.12"))?;
```

Request:

```rust
pub struct UseRequest {
    pub tools: Vec<ToolSpec>,
    pub scope: UseScope,
    pub pin: bool,
    pub force: bool,
    pub path: Option<Utf8PathBuf>,
    pub env_name: Option<String>,
}
```

Scope:

```rust
pub enum UseScope {
    Local,
    Global,
    Path(Utf8PathBuf),
    Env(String),
}
```

Support flags:

* global config
* path-specific config
* env-specific config
* pin exact version
* force reinstall
* fuzzy versions
* multiple tools in one call

### 16.2 Pin

Pin should be a first-class helper:

```rust
mise.pin(["node@lts", "npm@latest"])?;
```

Internally uses `mise use --pin`.

Purpose:

* resolve aliases like `latest`, `lts`, etc.
* write exact versions into config
* improve reproducibility

### 16.3 Unuse

Expose:

```rust
mise.unuse(["node"])?;
mise.unuse_global(["node"])?;
```

This removes tool entries from config.

Important distinction:

```text
uninstall = remove installed files
unuse = remove config reference
```

Both are needed.

## 17. Environment API

This is one of the most important parts for library consumers.

Expose:

```rust
let env = mise.env(EnvRequest::for_tools(["node@22"]))?;
```

Use `mise env --json` or `mise env --json-extended`.

Return:

```rust
pub struct MiseEnv {
    pub vars: BTreeMap<String, String>,
    pub extended: Option<Vec<EnvEntry>>,
}
```

Extended:

```rust
pub struct EnvEntry {
    pub key: String,
    pub value: String,
    pub source: Option<String>,
    pub tool: Option<String>,
}
```

Support:

* JSON output
* JSON extended
* dotenv output
* shell-specific output
* redacted output
* values-only output

### 17.1 Use cases

Rust app wants to spawn a Node script with Node 22:

```rust
let env = mise.env_for(["node@22"])?;
Command::new("node")
    .envs(env.vars)
    .arg("script.js")
    .status()?;
```

But preferably, use `mise exec`.

## 18. Exec API

Expose:

```rust
mise.exec()
    .tool("node@22")
    .arg("node")
    .arg("--version")
    .run()?;
```

Request:

```rust
pub struct ExecRequest {
    pub tools: Vec<ToolSpec>,
    pub command: Vec<String>,
    pub cwd: Option<Utf8PathBuf>,
    pub jobs: Option<usize>,
    pub fresh_env: bool,
    pub no_deps: bool,
    pub sandbox: Option<SandboxPolicy>,
}
```

Sandbox support should exist because mise has experimental flags for denying/allowing reads, writes, network, and env inheritance.

```rust
pub struct SandboxPolicy {
    pub deny_all: bool,
    pub deny_read: bool,
    pub deny_write: bool,
    pub deny_net: bool,
    pub deny_env: bool,
    pub allow_read: Vec<Utf8PathBuf>,
    pub allow_write: Vec<Utf8PathBuf>,
    pub allow_net: Vec<String>,
    pub allow_env: Vec<String>,
}
```

Caveat:

* mark sandbox APIs as experimental
* OS behavior differs
* Linux/macOS support may differ depending on mise behavior

## 19. Path Resolution API

Expose:

```rust
mise.where_tool("node@22")?;
mise.which("node")?;
mise.which_with_tool("npm", "node@22")?;
mise.which_version("node")?;
mise.which_plugin("node")?;
```

Types:

```rust
pub struct ToolInstallPath {
    pub path: Utf8PathBuf,
}

pub struct BinResolution {
    pub bin: String,
    pub path: Option<Utf8PathBuf>,
    pub version: Option<String>,
    pub plugin: Option<String>,
}
```

Important commands:

* `mise where <tool@version>`
* `mise which <bin>`
* `mise which <bin> --tool=<tool@version>`
* `mise which <bin> --version`
* `mise which <bin> --plugin`

This gives app UIs a way to show exactly which binary will run.

## 20. Config API

### 20.1 Read config

Expose:

```rust
mise.config_ls()?;
mise.config_get("tools.node")?;
```

Support:

* config hierarchy
* local/global/project config
* `mise.toml`
* `mise.local.toml`
* environment configs
* `.mise/config.toml`
* `.config/mise.toml`
* `conf.d/*.toml`

### 20.2 Write config

Expose:

```rust
mise.config_set("tools.node", "22")?;
mise.config_set_typed("settings.jobs", 4)?;
mise.config_set_list("settings.disable_tools", ["node", "rust"])?;
```

Use `mise config set` when possible.

Do not home-cook config mutation unless needed.

If direct TOML editing is needed:

* use `toml_edit`
* preserve formatting where possible
* only edit known keys
* atomic write through temp file + rename
* never silently reorder whole file
* validate with `mise config ls` after write

### 20.3 Strongly typed config model

```rust
pub struct MiseToml {
    pub tools: BTreeMap<String, ToolConfig>,
    pub env: BTreeMap<String, String>,
    pub settings: MiseSettings,
    pub tasks: BTreeMap<String, TaskConfig>,
}
```

But keep raw TOML access available.

Mise config evolves. The crate should not reject unknown keys.

Use:

```rust
#[serde(flatten)]
pub extra: BTreeMap<String, toml::Value>
```

## 21. Settings API

Expose:

```rust
mise.settings()?;
mise.settings_all()?;
mise.settings_local()?;
mise.settings_get("python")?;
mise.settings_set("jobs", "8")?;
mise.settings_unset("jobs")?;
```

Support:

* JSON output
* TOML output
* extended source output
* local/global distinction

Settings matter for:

* lockfiles
* jobs
* idiomatic version files
* tool disabling
* language-specific behavior
* Ruby compile mode
* Python options
* Java shorthand vendor
* Rust cargo/rustup isolation behavior

## 22. Lockfile API

Expose:

```rust
mise.lock()?;
mise.lock_tool("node")?;
mise.lock_platforms(["linux-x64", "macos-arm64"])?;
mise.lock_dry_run()?;
```

Request:

```rust
pub struct LockRequest {
    pub tools: Vec<String>,
    pub global: bool,
    pub local_lock: bool,
    pub platforms: Vec<String>,
    pub jobs: Option<usize>,
    pub dry_run: bool,
    pub minimum_release_age: Option<String>,
}
```

Also expose:

```rust
mise.install_locked()?;
```

This sets `--locked` or environment/settings equivalent.

Purpose:

* reproducible installs
* no surprise upstream resolution
* better CI behavior
* safer deployment environments
* fewer API calls to GitHub/aqua/etc.

## 23. Outdated / Upgrade API

### 23.1 Outdated

Expose:

```rust
mise.outdated()?;
mise.outdated_tool("node")?;
mise.outdated_local()?;
mise.outdated_inactive()?;
```

Return:

```rust
pub struct OutdatedTool {
    pub plugin: String,
    pub requested: String,
    pub current: String,
    pub latest: String,
}
```

Support:

* JSON output
* bump mode
* inactive mode
* local-only mode

### 23.2 Upgrade

Expose:

```rust
mise.upgrade()?;
mise.upgrade_tools(["node@22", "python@3.12"])?;
mise.upgrade_bump()?;
mise.upgrade_dry_run()?;
```

Request:

```rust
pub struct UpgradeRequest {
    pub tools: Vec<ToolSpec>,
    pub bump: bool,
    pub dry_run: bool,
    pub dry_run_code: bool,
    pub inactive: bool,
    pub exclude: Vec<String>,
    pub jobs: Option<usize>,
}
```

Important behavior:

* default upgrade should respect the range in `mise.toml`
* `--bump` can update the config to a newer major/minor range
* dry-run must be exposed for safe app previews

## 24. Prune API

Expose:

```rust
mise.prune_dry_run()?;
mise.prune_tools()?;
mise.prune_configs()?;
```

Request:

```rust
pub struct PruneRequest {
    pub tools: Vec<String>,
    pub dry_run: bool,
    pub dry_run_code: bool,
    pub only_tools: bool,
    pub only_configs: bool,
}
```

Return dry-run deletion candidates:

```rust
pub struct PrunePlan {
    pub paths: Vec<Utf8PathBuf>,
}
```

This is needed for cleanup UIs.

## 25. Cache API

Expose:

```rust
mise.cache_clear()?;
mise.cache_clear_tools(["node", "python"])?;
mise.cache_path()?;
mise.cache_prune()?;
```

Purpose:

* refresh version listings
* recover from bad downloads
* force new upstream metadata
* cleanup disk usage

## 26. Doctor / Diagnostics API

Expose:

```rust
mise.doctor()?;
mise.doctor_path()?;
```

Return:

```rust
pub struct DoctorReport {
    pub ok: bool,
    pub raw_output: String,
    pub warnings: Vec<Diagnostic>,
    pub errors: Vec<Diagnostic>,
}
```

Initially this can be raw-text parsed lightly.

Later, if mise adds structured JSON for doctor, switch to JSON.

Diagnostics should include wrapper-side checks too:

* mise binary exists
* mise version supported
* PATH contains mise
* project config found
* config trusted or not
* lockfile status
* missing tools
* outdated tools
* invalid config
* tool executable resolution
* shell activation presence
* OS dependencies warning for Ruby/Python compile flows
* network/proxy env availability
* permissions on mise data/cache dirs

## 27. Trust / Security API

Mise has trust/untrust concepts around config.

Expose:

```rust
mise.trust(path)?;
mise.untrust(path)?;
```

Also provide safe defaults:

```rust
pub struct SecurityPolicy {
    pub allow_hooks: bool,
    pub allow_env_from_config: bool,
    pub locked: bool,
    pub require_trusted_config: bool,
    pub minimum_release_age: Option<String>,
}
```

For hostile/untrusted project directories, allow:

```rust
mise.no_config()
mise.no_env()
mise.no_hooks()
mise.locked()
```

This is important for AI agents opening random repos.

Default wrapper policy should be conservative for automation:

```text
local trusted project mode:
  config/env/hooks allowed

untrusted repo mode:
  --no-hooks
  optionally --no-env
  maybe require explicit trust before install
```

## 28. Language-Specific Convenience Layer

The crate should have generic APIs first, then small typed helpers.

### 28.1 Node.js

Capabilities:

* install Node version
* use global/local Node
* pin Node and npm
* list remote Node versions
* resolve `node`, `npm`, `npx`, `corepack`
* support npm as separate tool when pinned

API:

```rust
mise.node().install("22")?;
mise.node().use_global("22")?;
mise.node().pin_lts_with_npm_latest()?;
```

Important behavior:

* Node can be managed like nvm/fnm/volta replacement.
* npm can be pinned separately.
* Do not run `npm install -g` from this crate unless using `npm:` backend intentionally.

### 28.2 Bun

Capabilities:

* install Bun
* list Bun versions
* use Bun globally/locally
* resolve `bun`

Important caveat:

* Do not use `bun upgrade` behind mise’s back.
* All upgrades should go through mise.

### 28.3 Deno

Capabilities:

* install Deno
* list Deno versions
* use Deno globally/locally
* resolve `deno`

Important caveat:

* Do not use `deno upgrade` behind mise’s back.
* All upgrades should go through mise.

### 28.4 Go

Capabilities:

* install Go version
* support `prefix:` behavior for older minor versions
* support `.go-version` if idiomatic files are enabled
* install Go CLIs using `go:` backend

API:

```rust
mise.go().use_global("1.23")?;
mise.go().install_cli("github.com/jesseduffield/lazygit", "latest")?;
```

Important:

* `go:` backend requires Go to compile the tool.
* For prebuilt binaries, prefer `aqua` or `github` when available.
* Do not recreate gvm behavior.

### 28.5 Python

Capabilities:

* install CPython versions
* install multiple Python versions
* support `.python-version`
* support `.python-versions`
* support virtualenv behavior through mise config
* integrate with uv behavior where mise supports it

API:

```rust
mise.python().use_local("3.12")?;
mise.python().use_multiple_global(["3.11", "3.12"])?;
```

Important:

* Do not become pip/venv manager unless explicitly scoped.
* Python package tools should use `pipx:` or `uv`/project tooling outside this core crate.
* Keep runtime management separate from dependency management.

### 28.6 Rust

Capabilities:

* select Rust toolchain through mise
* support stable/beta/nightly/specific versions
* support components
* support targets
* understand that rustup still manages actual toolchains

API:

```rust
mise.rust().use_global("stable")?;
mise.rust().use_local("1.82")?;
mise.rust().with_components("1.83.0", ["rust-src", "llvm-tools"])?;
```

Important:

* Rust is special because mise uses rustup under the hood.
* Installs may not live under normal mise install paths.
* Do not replace rustup.
* Allow isolation via env like `MISE_RUSTUP_HOME` and `MISE_CARGO_HOME`.

### 28.7 Ruby

Capabilities:

* install Ruby versions
* use global/local Ruby
* support compiled or precompiled behavior through settings
* warn about OS build dependencies when compiling

API:

```rust
mise.ruby().use_global("3.3")?;
mise.ruby().set_precompiled(true)?;
```

Important:

* Ruby builds can fail due to missing system packages.
* Wrapper should classify this as `DependencyMissing` when possible.
* Do not implement ruby-build yourself.

### 28.8 Java

Capabilities:

* install Java by vendor/version
* support OpenJDK shorthand
* support vendors like Temurin, Zulu, Corretto
* resolve `JAVA_HOME` from generated env

API:

```rust
mise.java().use_global("temurin-21")?;
mise.java().java_home()?;
```

Important:

* Shims alone may not set `JAVA_HOME`; generated env / activation matters.

### 28.9 Generic tools

Everything else must still work.

```rust
mise.tool("terraform").use_local("1.9")?;
mise.tool("aws-cli").install("latest")?;
mise.tool("github:cli/cli").install("latest")?;
mise.tool("aqua:aws/aws-cli").install("latest")?;
mise.tool("cargo:ripgrep").install("latest")?;
mise.tool("npm:prettier").install("3")?;
mise.tool("pipx:black").install("latest")?;
mise.tool("gem:rubocop").install("latest")?;
```

This is the real power.

Do not overfit to languages.

## 29. Backend Coverage

The crate should understand backend prefixes:

```text
core:
asdf:
aqua:
cargo:
conda:
dotnet:
gem:
github:
gitlab:
go:
http:
npm:
pipx:
spm:
ubi:
vfox:
```

But it should not need custom logic for every backend.

Model them as strings.

Add typed helpers only where useful.

Important backend policy:

* prefer core tools for major languages
* prefer aqua/github/gitlab for prebuilt binaries
* use cargo/npm/pipx/gem/go only when that ecosystem is actually required
* warn when a backend requires a runtime already installed
* expose backend security info where mise provides it

## 30. Mise Binary Management

The crate should not assume `mise` is installed.

Provide:

```rust
MiseBinary::discover()
MiseBinary::from_path(path)
MiseBinary::ensure_installed()
```

### 30.1 Discovery order

```text
explicit path passed by user
MISE_BIN env var
PATH lookup with which
well-known install paths
app-bundled binary path
```

### 30.2 Installing mise

This is optional and should be behind a feature:

```toml
features = ["bootstrap"]
```

Options:

* tell user how to install mise
* download release binary
* use package manager hints
* use self-contained app bundle

Do not silently curl-shell-install in library code.

A library should never surprise-install global tools unless explicitly asked.

## 31. Version Compatibility

On client creation:

```rust
mise --version
```

Parse version.

Support policy:

```text
minimum supported mise version: configurable by crate release
warn on old versions
error on unsupported versions if strict mode enabled
```

Types:

```rust
pub struct MiseVersion {
    pub raw: String,
    pub parsed: Option<semver::Version>,
}
```

Feature:

```rust
Mise::capabilities()
```

Returns:

```rust
pub struct MiseCapabilities {
    pub json_ls: bool,
    pub json_env: bool,
    pub registry_security: bool,
    pub lockfile: bool,
    pub sandbox_exec: bool,
}
```

This prevents brittle assumptions.

## 32. Config Safety

Writing config is dangerous if sloppy.

Rules:

* prefer `mise use` and `mise config set`
* only direct-edit TOML for features mise CLI cannot express
* preserve unknown keys
* preserve comments when direct-editing via `toml_edit`
* write atomically
* validate after write
* never delete user config without explicit request
* distinguish global/local/env/path config
* surface target file in result

Return write result:

```rust
pub struct ConfigWriteResult {
    pub file: Utf8PathBuf,
    pub changed: bool,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}
```

## 33. Execution Safety

Never build shell strings.

Bad:

```rust
format!("mise exec {} -- {}", tool, command)
```

Good:

```rust
cmd.arg("exec")
   .arg(tool)
   .arg("--")
   .args(command)
```

Also:

* validate tool specs
* reject empty command
* no shell by default
* support shell execution only explicitly
* redact env secrets
* timeout support
* stream output optionally
* capture output optionally
* allow cancellation in async mode

## 34. Output Modes

Some consumers need captured output; some need streaming.

Expose:

```rust
ExecOutputMode::Capture
ExecOutputMode::Stream
ExecOutputMode::Inherit
```

Result:

```rust
pub struct ExecResult {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
}
```

For streaming, return status and optionally collected tail.

## 35. Hooks and Env Handling

Mise can load environment variables and hooks from config.

Wrapper options:

```rust
pub struct LoadPolicy {
    pub config: bool,
    pub env: bool,
    pub hooks: bool,
}
```

Mapping:

```text
config=false => --no-config
env=false    => --no-env
hooks=false  => --no-hooks
```

Default:

* for normal trusted project use: allow all
* for automation over unknown repos: no hooks until trusted

## 36. Project Context

A central concept:

```rust
pub struct MiseProject {
    pub root: Utf8PathBuf,
    pub mise: Mise,
}
```

Capabilities:

```rust
project.detect_config_files()?;
project.list_tools()?;
project.install_missing()?;
project.env()?;
project.exec(["node", "--version"])?;
project.lock()?;
```

This is useful for IDEs/agents.

## 37. Dynamic Runtime Manager API

For app-level use, expose a generic manager:

```rust
pub struct RuntimeManager {
    mise: Mise,
}
```

Methods:

```rust
manager.ensure("node", "22")?;
manager.ensure_many([
    ("node", "22"),
    ("bun", "latest"),
    ("python", "3.12"),
])?;

manager.run_with("node@22", ["node", "--version"])?;
manager.resolve_bin("node")?;
```

This gives the high-level UX you want.

## 38. Example Public API

### 38.1 Install Node and run npm

```rust
let mise = Mise::discover()?;

mise.use_tool(UseRequest::local("node@22"))?;

let result = mise.exec(ExecRequest::new()
    .tool("node@22")
    .command(["npm", "install"]))?;
```

### 38.2 Ensure Python and get env

```rust
let mise = Mise::discover()?;

mise.install(["python@3.12"])?;

let env = mise.env(EnvRequest::new()
    .tool("python@3.12")
    .json_extended(true))?;
```

### 38.3 Global Bun

```rust
mise.use_tool(UseRequest::global("bun@latest"))?;
```

### 38.4 Rust with components

```rust
mise.use_tool(
    UseRequest::global(
        ToolSpec::new("rust")
            .version("1.83.0")
            .option("components", "rust-src,llvm-tools")
    )
)?;
```

### 38.5 Check project health

```rust
let report = mise.diagnostics()
    .check_binary()
    .check_config()
    .check_missing_tools()
    .check_outdated()
    .run()?;
```

## 39. Feature Flags

Recommended crate features:

```toml
default = ["sync", "json"]

sync = ["duct"]
async = ["tokio/process"]
json = ["serde", "serde_json"]
toml = ["toml_edit", "toml"]
diagnostics = []
bootstrap = ["reqwest"]
tracing = ["dep:tracing"]
miette = ["dep:miette"]
```

Avoid pulling heavy deps by default.

## 40. Dependencies

Recommended dependencies:

```toml
duct = "0.13"
which = "8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.9"
toml_edit = "0.23"
camino = { version = "1", features = ["serde1"] }
fs-err = "3"
thiserror = "2"
tracing = "0.1"
tempfile = "3"
semver = "1"
shell-words = "1"
```

Optional:

```toml
tokio = { version = "1", features = ["process", "io-util", "macros"], optional = true }
miette = { version = "7", optional = true }
reqwest = { version = "0.12", optional = true }
```

## 41. Testing Strategy

### 41.1 Unit tests

Use fake runner.

Test:

* command construction
* args escaping
* JSON parsing
* error classification
* config model
* tool spec parsing
* path handling
* lockfile request construction
* registry parsing
* env parsing

### 41.2 Snapshot tests

Use `insta`.

Snapshot:

* generated command args
* parsed JSON fixtures
* error outputs
* diagnostics report rendering

### 41.3 Integration tests

Require real mise.

Gate behind:

```text
MISE_MANAGER_INTEGRATION=1
```

Tests:

* `mise --version`
* `mise registry --json`
* `mise ls --json`
* `mise env --json`
* install tiny/fast tools only
* avoid massive downloads by default

### 41.4 Expensive tests

Gate behind:

```text
MISE_MANAGER_EXPENSIVE=1
```

Can test:

* install Node
* install Bun
* install Deno
* install Python
* install Go
* install Ruby only in CI image with dependencies

### 41.5 CI Matrix

Test on:

* Linux x86_64
* Linux arm64 if possible
* macOS arm64
* macOS x86_64 if available
* Windows later, if path and shell behavior is supported

## 42. Fixture Strategy

Keep JSON fixtures from real mise output:

```text
fixtures/
  ls/
    installed.json
    missing.json
    current.json
  ls_remote/
    node.json
    github_cli.json
  env/
    basic.json
    extended.json
  registry/
    basic.json
    security.json
  outdated/
    basic.json
  settings/
    all.json
```

Tests should not depend on current upstream versions.

## 43. Error Classification Heuristics

Mise stderr can be human text.

Create a conservative classifier:

```rust
pub enum FailureKind {
    BinaryMissing,
    ToolUnknown,
    VersionUnknown,
    Network,
    Checksum,
    Permission,
    DependencyMissing,
    ConfigUntrusted,
    ConfigInvalid,
    LockfileMissing,
    CommandFailed,
    Unknown,
}
```

Start conservative.

Do not over-parse.

Expose raw stderr always.

## 44. Security Considerations

Important for AI agents and automation:

* never execute hooks from unknown repos unless allowed
* support `--no-hooks`
* support `--no-env`
* support `--no-config`
* support `--locked`
* support minimum release age
* support lockfiles
* redact secrets in logs
* never run shell strings by default
* do not trust project config automatically in agent workflows
* show dry-run plans before destructive actions
* require explicit confirmation for prune/uninstall helpers at app level
* do not silently install mise itself
* avoid invoking language-native self-updaters like `bun upgrade`, `deno upgrade`, etc.

## 45. Destructive Operation Policy

Destructive operations:

* uninstall
* unuse
* prune
* cache clear
* config overwrite
* settings unset
* lockfile overwrite

For library API:

* allow direct calls
* return detailed result
* provide dry-run helpers where mise supports them
* never prompt inside the library
* prompting belongs to the app/CLI layer

## 46. Documentation Plan

Docs should be practical.

Pages:

```text
README.md
docs/
  architecture.md
  why-wrapper-not-vendor.md
  installation.md
  quickstart.md
  tool-management.md
  project-config.md
  running-commands.md
  environments.md
  lockfiles.md
  diagnostics.md
  security.md
  language-node.md
  language-bun.md
  language-deno.md
  language-go.md
  language-python.md
  language-rust.md
  language-ruby.md
  language-java.md
  testing.md
```

README should include:

* crate goal
* not a mise replacement
* requirements
* basic install/use example
* project-local example
* exec example
* diagnostics example
* security note for untrusted repos

## 47. MVP Scope

MVP should not try to cover everything.

### MVP commands

* discover mise binary
* `mise --version`
* `mise registry --json`
* `mise ls --json`
* `mise ls-remote --json`
* `mise install`
* `mise uninstall`
* `mise use`
* `mise unuse`
* `mise env --json`
* `mise exec`
* `mise where`
* `mise which`
* `mise outdated --json`
* `mise prune --dry-run`
* `mise cache clear`
* `mise settings ls --json`
* `mise config set`
* `mise lock --dry-run`

### MVP language helpers

* generic
* node
* bun
* deno
* go
* python
* rust
* ruby

### MVP must-have safety

* no shell strings
* typed errors
* dry-run support
* timeout support
* no-hooks/no-env/no-config support
* locked support
* redacted logging

## 48. V1 Scope

After MVP:

* async feature
* streaming output
* richer doctor parsing
* lockfile model
* settings set/unset typed API
* config get/list typed API
* registry security parsing
* shell activation helpers
* Java helper
* tool alias APIs
* plugin APIs
* backend listing APIs
* task APIs if needed

## 49. V2 Scope

Later:

* mise bootstrap/install feature
* app-bundled mise binary support
* Windows polish
* richer config merge model
* UI-friendly progress events
* structured install progress parsing
* sandbox policy API
* MCP integration wrapper if useful
* OCI experimental wrappers if needed
* dependency APIs if mise deps become central to our use case

## 50. What We Should Not Build

Do not build:

* Node installer
* Python installer
* Ruby compiler
* Go bootstrapper
* Rustup replacement
* dependency manager
* package manager UI
* shell framework
* plugin system clone
* version feed scraper
* download cache
* checksum database
* custom registry
* asdf plugin runner

Let mise do all of that.

## 51. Open Questions

Before implementation:

1. Should the crate require mise to already be installed?
2. Should bootstrap be a separate crate?
3. Should async be included in MVP or later?
4. Should command execution stream progress in MVP?
5. Should we support Windows in MVP?
6. Should the library expose task APIs now or later?
7. Should we support project trust APIs in MVP?
8. Should we parse TOML directly or only use mise config commands?
9. Should we expose language helpers as feature flags?
10. Should we pin a minimum mise version?

Recommended answers:

1. Require mise installed in MVP.
2. Bootstrap later, optional feature.
3. Sync MVP, async later.
4. Capture MVP, stream later.
5. Linux/macOS MVP, Windows later.
6. Tasks later.
7. Basic trust/no-hooks policy in MVP.
8. Use mise config commands first, TOML direct-edit only where needed.
9. Keep helpers included but lightweight.
10. Yes, define and test minimum supported mise version.

## 52. Final Architecture

```text
Consumer Rust App
    |
    | calls typed Rust API
    v
mise-manager crate
    |
    | validates requests
    | builds args safely
    | runs through duct/tokio adapter
    | parses JSON where possible
    | maps errors
    v
mise binary
    |
    | uses core tools / registry / backends / config / lockfile
    v
installed tools and language runtimes
```

## 53. Final Recommendation

Build the crate as a typed mise engine wrapper.

Do not vendor mise internals.

Do not recreate language managers.

Do not call language-native upgrade/install flows behind mise’s back.

Use mise’s documented CLI, JSON outputs, lockfiles, config system, registry, and backend architecture.

The value of our crate is not “installing Node better than mise.”

The value is:

```text
safe Rust API
typed requests
typed results
good errors
diagnostics
project automation
runtime execution
agent-friendly security
multi-language abstraction
```

That is the right abstraction layer.
