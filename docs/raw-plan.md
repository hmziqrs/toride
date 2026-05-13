# Product Goal

Build a Rust + Ratatui terminal application that guides a user through VPS setup, security hardening, developer tooling installation, deployment stack installation, and quality-of-life configuration.

The app should not be a random shell-script runner. It should feel like a guided installer with profiles, checklists, warnings, dependency handling, dry-runs, logs, rollback notes, and reusable recipes.

The core idea:

> Select a profile → review modules → configure options → run preflight checks → apply setup → show summary, logs, next steps, and generated restore/debug commands.

---

# Target Users

## Primary users

* Developers setting up fresh VPS servers
* Indie hackers deploying apps
* Agencies preparing client servers
* Homelab users
* AI-agent users who want repeatable server environments
* People deploying Dokploy, Coolify, Traefik, Caddy, Docker, Node, Bun, Rust, Go, and similar stacks

## Supported VPS environments

Initial target:

* Debian 12
* Ubuntu 22.04 LTS
* Ubuntu 24.04 LTS

Later:

* Debian 13
* AlmaLinux / Rocky Linux
* Fedora Server
* Arch-based servers only as experimental

---

# Core Profiles

## 1. Basic Profile

For a normal production VPS.

Preselected modules:

* System update
* Create non-root sudo user
* Add SSH key
* Disable root SSH login
* Disable password SSH login after key verification
* UFW firewall
* Fail2Ban
* Docker
* Docker Compose plugin
* Swap
* Basic system utilities
* Timezone / locale setup
* Unattended security upgrades
* Basic monitoring tools

Optional:

* Node.js
* Bun
* Rust
* Go
* Python tooling
* Tailscale
* Caddy or Traefik

## 2. Sandbox Profile

For testing, experiments, temporary VPS instances, AI-agent playgrounds, or unsafe prototypes.

Preselected modules:

* System update
* Create non-root sudo user
* SSH key setup
* UFW firewall
* Docker
* Docker Compose plugin
* Node.js
* Bun
* Deno
* Rust
* Go
* Python tooling
* Tailscale
* Swap
* Dev utilities

Less strict by default:

* Root login can remain enabled until the user confirms key-based access works
* Password login can remain temporarily enabled
* Cloudflare-only HTTP/S disabled by default
* Fail2Ban optional

## 3. Custom Profile

User manually selects everything.

The app should still warn about unsafe combinations, missing dependencies, and conflicting choices.

Examples:

* Dokploy requires Docker
* Coolify requires Docker
* Docker-based Traefik should require Docker
* Cloudflare-only HTTP/S requires the user to confirm they actually use Cloudflare proxy
* Disabling password SSH login requires SSH key validation first

---

# Main Menu Structure

```text
VPS Setup
├─ Profiles
│  ├─ Basic
│  ├─ Sandbox
│  └─ Custom
│
├─ System Basics
│  ├─ System update / upgrade
│  ├─ Hostname
│  ├─ Timezone
│  ├─ Locale
│  ├─ Swap
│  ├─ Essential packages
│  └─ Reboot check
│
├─ Users & SSH
│  ├─ Create sudo user
│  ├─ Add SSH key
│  ├─ Disable root login
│  ├─ Disable password login
│  ├─ Change SSH port
│  ├─ SSH config validation
│  └─ Emergency rollback instructions
│
├─ Firewall & Security
│  ├─ UFW
│  ├─ Fail2Ban
│  ├─ Cloudflare-only HTTP/S
│  ├─ Rate limiting
│  ├─ Automatic security updates
│  ├─ Kernel hardening sysctl
│  └─ Audit tools
│
├─ Developer Runtimes
│  ├─ Node.js
│  ├─ Bun
│  ├─ Deno
│  ├─ Rust
│  ├─ Go
│  ├─ Python
│  └─ Java / JDK optional
│
├─ Containers
│  ├─ Docker
│  ├─ Docker Compose plugin
│  ├─ Docker user permissions
│  ├─ Docker log rotation
│  └─ Docker daemon config
│
├─ Server Managers
│  ├─ Dokploy
│  ├─ Coolify
│  └─ None
│
├─ Reverse Proxy
│  ├─ Caddy native
│  ├─ NGINX native
│  ├─ Traefik native
│  ├─ Caddy in Docker
│  ├─ NGINX in Docker
│  ├─ Traefik in Docker
│  └─ Let server manager handle it
│
├─ Networking
│  ├─ Tailscale
│  ├─ Cloudflare Tunnel
│  ├─ WireGuard optional
│  └─ DNS helpers
│
├─ Quality of Life
│  ├─ zsh / fish optional
│  ├─ starship prompt optional
│  ├─ tmux
│  ├─ htop / btop
│  ├─ ncdu
│  ├─ jq
│  ├─ ripgrep
│  ├─ fd
│  ├─ git
│  ├─ curl / wget
│  └─ log viewer helpers
│
├─ Observability
│  ├─ Node exporter
│  ├─ Prometheus optional
│  ├─ Grafana optional
│  ├─ Netdata optional
│  ├─ Uptime Kuma optional
│  └─ Log rotation
│
└─ Run Plan
   ├─ Preflight check
   ├─ Dry run
   ├─ Apply
   ├─ Save logs
   └─ Export setup report
```

---

# Service Selection List

Base checklist:

```text
[ ] Docker
[ ] Node.js
[ ] Bun
[ ] Deno
[ ] Rust
[ ] Go
[ ] Python
[ ] UFW
[ ] Fail2Ban
[ ] Cloudflare-only HTTP/S
[ ] SSH hardening
[ ] Swap
[ ] Dokploy
[ ] Coolify
[ ] Traefik
[ ] Caddy
[ ] NGINX
[ ] Tailscale
[ ] Cloudflare Tunnel
[ ] Automatic security updates
[ ] Docker log rotation
[ ] Basic monitoring tools
[ ] Backup tools
```

---

# Important Service Grouping

## Language runtimes

Toride consolidates Node, Bun, Deno, Go, Rust, and Python under a single runtime manager rather than installing each via its own bespoke script. The v0.1 runtime manager is **mise** (`https://mise.jdx.dev`). It is a single static binary, handles all six languages, and avoids the `.bashrc` mutation that NVM-style installers depend on.

v0.1 option:

* mise

Future fallback:

* Per-language scripts may be added later only for explicit advanced fallback cases.

Recommended UX:

```text
Language Runtimes (managed by mise)
[x] Node.js  20 LTS
[x] Bun      latest
[ ] Deno
[x] Rust     stable
[x] Go       1.22
[x] Python   3.12

Install scope:
( ) System-wide (mise in /usr/local/bin, runtimes under /opt/mise)
(*) Per-user (mise under target sudo user)
```

Important rules:

* Install mise under the target sudo user, not root.
* Do not expose NVM, asdf, NodeSource, rustup, or Go tarball installs as first-class v0.1 runtime choices.
* Warn if `/usr/bin/node` from apt is present — coexisting versions confuse `which node`.
* If Bun is selected, still ask whether Node compatibility is needed.
* `rustup` may be reconsidered after v0.1 if mise's Rust behavior is not sufficient for real users.

## Server managers

Options:

* Dokploy
* Coolify
* None

Rules:

* Dokploy requires Docker.
* Coolify requires Docker.
* Do not install both by default.
* If one is selected, ask whether the reverse proxy should be managed by the server manager.

## Reverse proxy

Options:

* Caddy
* NGINX
* Traefik
* Docker-based Caddy
* Docker-based NGINX
* Docker-based Traefik
* Handled by Dokploy/Coolify

UX rule:

If Docker is selected, ask:

```text
Docker is enabled. How should reverse proxy be installed?

( ) Native system package
( ) Docker container
( ) Let Dokploy/Coolify manage it
( ) Skip reverse proxy
```

Conflict warnings:

* Avoid binding multiple services to ports 80 and 443.
* Warn if Dokploy/Coolify and standalone Traefik/Caddy are both selected.
* Warn if Cloudflare-only HTTP/S is enabled but the user does not use proxied Cloudflare DNS.

---

# Extra Essential VPS Modules

These are the useful things people commonly need beyond runtime installation.

## Security essentials

* Automatic security updates
* UFW firewall
* Fail2Ban
* SSH hardening
* Disable root SSH login
* Disable password SSH login
* SSH key install and validation
* Optional SSH port change
* Cloudflare-only HTTP/S firewall rules
* Docker daemon hardening
* System audit summary
* Reboot-required detection
* Basic sysctl hardening
* Login history summary
* Open ports summary

## Quality-of-life essentials

* git
* curl
* wget
* unzip
* tar
* jq
* ripgrep
* fd-find
* nano / vim / neovim
* tmux
* htop / btop
* ncdu
* lsof
* net-tools or modern alternatives
* dnsutils
* ca-certificates
* gnupg
* build-essential
* pkg-config

## Reliability essentials

* Swap file creation
* Docker log rotation
* journald log limits
* unattended-upgrades
* time sync
* disk usage warnings
* memory check
* reboot-required check
* service status report

## Backup essentials

* Restic
* BorgBackup
* Rclone
* Database dump helpers
* Docker volume backup helper
* Cron/systemd timer setup
* Backup restore instructions

## Monitoring essentials

* Uptime Kuma
* Netdata
* Prometheus node_exporter
* Grafana optional
* Docker container health summary
* Disk usage summary
* Failed services summary

## Networking essentials

* Tailscale
* Cloudflare Tunnel
* WireGuard optional
* DNS check tools
* Public IP detection
* IPv6 detection
* Open port testing
* Firewall rule summary

---

# Security Flow

Security must be done carefully to avoid locking the user out.

## Safe SSH hardening flow

1. Detect current SSH connection (parse `SSH_CONNECTION` and `who`).
2. Detect current user and whether the session is over SSH.
3. Ask for new username.
4. Create user with home directory.
5. Add user to sudo group; install `/etc/sudoers.d/00-toride-<user>` with optional NOPASSWD.
6. Add SSH public key to `/home/<user>/.ssh/authorized_keys` (file 600, dir 700, correct ownership).
7. Validate authorized_keys permissions and SELinux context where applicable.
8. Show a client reconnect command using the detected server address, for example `ssh <user>@<server-ip> true`.
9. Require the operator to open a second terminal from their own machine, run the command, and explicitly confirm success in Toride.
10. Treat this confirmation as the gate for disabling root or password login. A same-host SSH check to `127.0.0.1` may be offered as a diagnostic only, but it is not sufficient proof that the operator can reconnect.
11. Write hardening to `/etc/ssh/sshd_config.d/00-toride.conf` (never edit the main `sshd_config` directly — both Debian 12 and Ubuntu 24.04 ship the `Include` line).
12. Detect `50-cloud-init.conf`. If it sets `PasswordAuthentication yes`, override it by removing the file or commenting that line — OpenSSH uses the **first** value found, so a later drop-in alone is not sufficient.
13. Only then offer to disable root login (`PermitRootLogin no`).
14. Only then offer to disable password login (`PasswordAuthentication no`, `KbdInteractiveAuthentication no`).
15. Validate sshd config using `sshd -t` before applying.
16. Reload SSH via `systemctl reload ssh` (reload, not restart — keeps the active session alive).
17. Show emergency rollback command (path to backup + `systemctl reload ssh`).

The app must never blindly disable root/password login before the operator confirms client-side reconnect success.

---

# Cloudflare-only HTTP/S Flow

This feature should restrict ports 80 and 443 to Cloudflare IP ranges only.

UX:

```text
Cloudflare-only HTTP/S protects your origin server by allowing only Cloudflare IP ranges to reach ports 80/443.

Use this only if your domain is proxied through Cloudflare.

Enable?
[ ] Yes
[ ] No
```

Implementation notes:

* Fetch IPv4 ranges from `https://www.cloudflare.com/ips-v4` and IPv6 from `https://www.cloudflare.com/ips-v6`. As of 2026-05 there are ~22 ranges total.
* Alternative endpoint with JSON metadata: `https://api.cloudflare.com/client/v4/ips`.
* Cache the result to `/var/lib/toride/cloudflare-ips.txt` with a fetch timestamp.
* Apply UFW rules for 80/443 allowing only those ranges; deny other HTTP/S traffic.
* Provide `toride refresh cloudflare-ips` subcommand and an optional weekly systemd timer.
* On refresh failure (network down, endpoint change), keep prior rules — never wipe rules on failed fetch.

Important warning:

If the user does not use Cloudflare proxied DNS, their website may become unreachable.

Future support:

* Bunny CDN IP allowlist
* Fastly IP allowlist
* CloudFront IP allowlist
* Custom CDN IP allowlist

---

# Runtime Requirements

## Privilege model

Toride separates planning from applying.

Planning may run without root:

* `toride plan --profile basic`
* `toride plan --config toride.toml`
* Interactive TUI up to the final Apply step

Apply requires root:

* `sudo toride apply --profile basic`
* `sudo toride apply --config toride.toml`
* Interactive TUI Apply step

Toride never spawns `sudo` per-command and never prompts for sudo after entering Ratatui raw mode. Sudo prompts corrupt raw-mode input and produce inconsistent state across modules. If the user reaches Apply without root, the app exits cleanly and shows the exact `sudo toride ...` command to rerun.

The `sudo` crate's `escalate_if_needed()` may be used only before the terminal enters raw mode, never after. `pkexec` is not a fit (headless VPS has no Polkit agent).

## Distribution and bootstrap

Toride ships as static `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` binaries published to GitHub Releases via `cargo-dist`.

Bootstrap one-liner for fresh VPSes:

```bash
curl -fsSL https://toride.dev/install.sh | sh
```

The install script verifies the release SHA256 against a published manifest before extracting. Developers may also use `cargo binstall toride` or `cargo install toride`.

## Preflight gotchas

Common fresh-VPS failure modes the executor must handle before module dispatch:

1. **Cloud-init still running** — On Ubuntu cloud images, run `cloud-init status --wait` if `cloud-init` is present. Skip on Debian images without it.
2. **apt-lock contention** — `unattended-upgrades` and `apt-daily.service` hold `/var/lib/dpkg/lock-frontend` for minutes after boot. Wrap all apt calls in `flock -w 600 /var/lib/dpkg/lock-frontend`.
3. **systemd absent** — LXC/OpenVZ containers and minimal Debian may lack systemd. Fail clearly; do not silently skip service modules.
4. **Pre-existing nftables rules** — Debian 12 defaults to nftables (empty). UFW uses the nftables backend transparently, but provider-preloaded rules are not visible via `ufw status`. Detect and warn before `ufw enable`.
5. **Reboot required** — If `/var/run/reboot-required` exists after package operations, surface it in the summary screen.

## Distro-specific module behavior

Non-obvious behaviors module authors must encode:

* **fail2ban on Ubuntu 24.04 / Debian 12** — SSH logs go to systemd journal, not `/var/log/auth.log`. The default backend reads nothing and silently protects no SSH. Install `python3-systemd` and write `backend = systemd` under `[DEFAULT]` in `/etc/fail2ban/jail.local` before starting the service.
* **SSH cloud-init override** — see Security Flow step 10 above.
* **Docker repo codename** — derive via `. /etc/os-release && echo $VERSION_CODENAME`. Never hardcode `bookworm` / `jammy` / `noble`.
* **Tailscale install script** — pin via `TAILSCALE_VERSION` and print the script URL + SHA before piping to sh. Honor the safety rule against silent `curl|sh`.

## Telemetry

Toride performs no telemetry, analytics, or phone-home. Network access is limited to:

* OS package repositories (apt)
* Vendor endpoints needed by selected modules (Docker repo, Tailscale install script, Cloudflare IP list, mise plugins)

Every network call is logged with URL, response status, and SHA where applicable, and is listed in the dry-run plan.

---

# Architecture

## Rust crates

Suggested stack:

* `ratatui` for TUI
* `crossterm` with `event-stream` feature for terminal backend
* `tachyonfx` for terminal animation effects
* `tokio` (full features) for async event loop and process spawning
* `tokio-util` for `CancellationToken`
* `futures` for `FutureExt` / `StreamExt`
* `tokio-stream` with `io-util` feature for subprocess line streaming
* `clap` (derive) for CLI flags
* `serde` / `serde_json` / `toml` for config
* `color-eyre` for app-level errors and panic handler
* `thiserror` for typed errors
* `tracing` for logs
* `tracing-subscriber` with `env-filter` and `json` features
* `tracing-appender` for rolling log files and non-blocking log writers
* `reqwest` (rustls backend) for downloading install scripts / IP ranges
* `sha2` / `hex` for verifying install-script and binary checksums
* `which` for binary detection
* `nix` for Unix syscalls (euid, file modes, signals)
* `dirs` for config paths
* `sudo` for optional `escalate_if_needed()` before Apply enters raw mode
* `async-trait` for the module trait

## Internal modules

```text
src/
├─ main.rs
├─ app.rs
├─ tui/
│  ├─ mod.rs
│  ├─ event.rs
│  ├─ update.rs
│  ├─ state.rs
│  ├─ screens.rs
│  ├─ widgets.rs
│  ├─ forms.rs
│  ├─ confirm.rs
│  ├─ animations.rs
│  └─ theme.rs
├─ profiles/
│  ├─ mod.rs
│  ├─ basic.rs
│  ├─ sandbox.rs
│  └─ custom.rs
├─ modules/
│  ├─ mod.rs
│  ├─ docker.rs
│  ├─ node.rs
│  ├─ bun.rs
│  ├─ deno.rs
│  ├─ rust.rs
│  ├─ go.rs
│  ├─ python.rs
│  ├─ ssh.rs
│  ├─ ufw.rs
│  ├─ fail2ban.rs
│  ├─ cloudflare.rs
│  ├─ swap.rs
│  ├─ dokploy.rs
│  ├─ coolify.rs
│  ├─ caddy.rs
│  ├─ nginx.rs
│  ├─ traefik.rs
│  └─ tailscale.rs
├─ executor/
│  ├─ mod.rs
│  ├─ command.rs
│  ├─ plan.rs
│  ├─ dry_run.rs
│  └─ logs.rs
├─ system/
│  ├─ os_detect.rs
│  ├─ package_manager.rs
│  ├─ users.rs
│  ├─ services.rs
│  └─ ports.rs
└─ config/
   ├─ mod.rs
   └─ schema.rs
```

## TUI architecture

Toride is interactive-first. Running `toride` launches the guided TUI; flags are shortcuts for repeatability, automation, CI checks, and AI agents. Every flag-based flow should have an equivalent interactive path.

Use the Elm Architecture / TEA pattern:

* `App` owns durable state.
* `Action` represents user input, subprocess events, ticks, signal events, and navigation.
* `update(app, action)` is the only place that mutates application state.
* Screens render from state and emit actions; they do not directly run commands.
* Reusable widgets that need cursor/selection state use Ratatui's `StatefulWidget` pattern.

Terminal setup:

* Call `color_eyre::install()` before entering the TUI.
* Use `ratatui::init()` for raw mode, alternate screen, and panic-hook restore.
* Enable bracketed paste through `crossterm` directly after terminal initialization.
* Always restore the terminal before printing fatal errors or rerun instructions.

The event loop should merge:

* Crossterm input events from `EventStream`.
* Periodic tick events for progress, timers, and animations.
* Executor progress events from module application.
* Subprocess stdout/stderr lines.
* Shutdown signals.

Signal handling:

* Watch `tokio::signal::ctrl_c()`.
* On Unix, also watch `tokio::signal::unix::signal(SignalKind::terminate())`.
* Convert both into `Action::Quit` so normal quit confirmation, cleanup, logging, and terminal restore are shared.

Animation:

* Use `tachyonfx` as the animation engine, with `EffectManager` managing active effects.
* Prefer short, meaningful transitions: screen enter, modal open/close, warning emphasis, and apply-step completion.
* Use `fx::sequence`, `fx::coalesce`, `fx::dissolve`, and `fx::fade_from` where they fit.
* If exact easing enum names drift between crate versions, pick the closest current easing at implementation time.
* Custom animation behavior may implement `Shader`, but avoid custom effects unless built-in effects cannot express the UI state clearly.

Color and accessibility:

* Honor `NO_COLOR` by disabling decorative color and preserving semantic labels.
* Honor `FORCE_COLOR` when output is not a TTY but color is explicitly requested.
* Keep foreground/background contrast at WCAG AA or better.
* Avoid pure black backgrounds for large terminal surfaces because high-contrast halation makes long sessions harder to read.
* Do not rely on color alone for safety state; include text labels like `[WARN]`, `[FAILED]`, and `[BLOCKED]`.

Ratatui version notes:

* Target Ratatui v0.30 or newer.
* Account for the v0.30 workspace split when choosing imports.
* Prefer `Layout::try_areas` where a layout failure should be handled explicitly instead of panicking.

## Runtime event handling

Subprocess output must stream into the TUI without blocking rendering.

Recommended command pattern:

```rust
use tokio::io::AsyncBufReadExt;

let mut child = tokio::process::Command::new(cmd)
    .args(args)
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()?;

let stdout = child.stdout.take().expect("stdout piped");
let stderr = child.stderr.take().expect("stderr piped");

let stdout_lines = tokio_stream::wrappers::LinesStream::new(
    tokio::io::BufReader::new(stdout).lines(),
);
let stderr_lines = tokio_stream::wrappers::LinesStream::new(
    tokio::io::BufReader::new(stderr).lines(),
);
```

Merge stdout and stderr streams with `StreamExt::merge`, tag each line with its source, and forward lines as progress events. Command completion should include exit code, duration, and whether the command changed the system.

Logging:

* Use `tracing_appender::rolling::RollingFileAppender` for `/var/log/toride/setup.log`.
* Wrap file writers in `tracing_appender::non_blocking::NonBlocking`.
* Keep the non-blocking worker guard alive for the full process lifetime.
* Write structured action records to `/var/log/toride/actions.jsonl` separately from human-readable logs.

---

# Module Design

Every installable item is a module with the same lifecycle. `apply` runs asynchronously and streams progress events back to the TUI on a tokio `mpsc` channel — it must not block the event loop.

```rust
#[async_trait]
trait SetupModule: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn dependencies(&self) -> &'static [&'static str];
    fn conflicts(&self) -> &'static [&'static str];

    async fn preflight(&self, ctx: &Context) -> Result<PreflightResult>;
    async fn plan(&self, ctx: &Context) -> Result<Vec<Action>>;
    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> Result<ApplyOutcome>;
    async fn verify(&self, ctx: &Context) -> Result<VerifyResult>;
}

type ProgressTx = tokio::sync::mpsc::UnboundedSender<ProgressEvent>;

enum ProgressEvent {
    StepStart { action_idx: usize, label: String },
    StepLog   { action_idx: usize, line: String },
    StepDone  { action_idx: usize, exit_code: i32, duration_ms: u64 },
    StepFail  { action_idx: usize, error: String },
}
```

## The `Action` type

`plan()` returns an ordered `Vec<Action>` so dry-run output is structured (not opaque shell strings) and rollback can be derived mechanically.

```rust
enum Action {
    AptInstall    { packages: Vec<&'static str> },
    AptRepoAdd    { name: &'static str, key_url: String, sources_line: String, sha256: String },
    WriteFile     { path: PathBuf, content: String, mode: u32, backup: bool },
    AppendLine    { path: PathBuf, line: String, marker: &'static str },
    Systemctl     { unit: String, op: SystemctlOp }, // Enable, Start, Reload, Restart
    UfwRule       { rule: String },
    UserCreate    { name: String, groups: Vec<String>, shell: PathBuf },
    UserAddKey    { user: String, key: String },
    DownloadScript{ url: String, sha256: String, run_as: String, env: Vec<(String, String)> },
    Exec          { cmd: String, args: Vec<String>, env: Vec<(String, String)>, as_user: Option<String> },
}
```

Rules:

* Every `Action` variant has a deterministic `to_shell_preview()` for dry-run rendering.
* `WriteFile { backup: true }` produces a timestamped copy under `/var/backups/toride/`.
* `DownloadScript` must include `sha256` — silent `curl|sh` is forbidden by safety rules.
* All apt operations are serialized under a single `flock` guard on `/var/lib/dpkg/lock-frontend`.

The app generates a plan before applying anything.

Example:

```text
Plan:
1. Update apt package index
2. Install ca-certificates, curl, gnupg
3. Add Docker official repository
4. Install Docker Engine
5. Enable Docker service
6. Add user `deploy` to docker group
7. Configure Docker log rotation
8. Verify `docker version`
```

---

# Execution Modes

## Interactive TUI mode

Default:

```bash
toride
```

## Non-interactive mode

For automation and AI agents:

```bash
toride apply --profile basic --user deploy --ssh-key ~/.ssh/id_ed25519.pub
```

## Dry-run mode

```bash
toride plan --profile basic
```

## Export plan

```bash
toride plan --profile basic --export setup-plan.json
```

## Apply from config

```bash
toride apply --config toride.toml
```

---

# Config File Example

```toml
profile = "basic"

[user]
name = "deploy"
ssh_key_path = "~/.ssh/id_ed25519.pub"
passwordless_sudo = true

[security]
disable_root_login = true
disable_password_login = true
ufw = true
fail2ban = true
cloudflare_only_http = false
auto_security_updates = true

[runtimes]
node = true
bun = true
deno = false
rust = true
go = true
python = true

[containers]
docker = true
docker_log_rotation = true

[server_manager]
manager = "dokploy"

[reverse_proxy]
mode = "managed_by_server_manager"

[networking]
tailscale = false
cloudflare_tunnel = false

[swap]
enabled = true
size = "2G"
```

---

# TUI Screens

## 1. Welcome

Show:

* App name
* Detected OS
* Current user
* Root/non-root status
* Public IP
* Memory
* Disk
* Existing Docker/Node/etc detection

## 2. Profile Selection

```text
Choose setup profile:

> Basic       Secure production-ready VPS setup
  Sandbox     Developer playground with common runtimes
  Custom      Manually choose every module
```

## 3. Module Selection

Checklist with categories and search/filter.

Controls:

```text
Space: toggle
Enter: configure
/: search
Tab: next category
r: reset profile defaults
p: preview plan
q: quit
```

## 4. Configuration Forms

Examples:

* Username
* SSH public key
* Swap size
* Runtime versions managed by mise
* Reverse proxy mode
* Server manager choice
* Cloudflare-only HTTP/S confirmation

Validation:

* Username: lowercase Linux account name, starts with a letter or underscore, no spaces, not `root`, not already present unless reusing intentionally.
* SSH public key: must parse as a supported public key format; reject private keys and empty input.
* Swap size: accepts explicit sizes like `512M`, `2G`, or `0` to disable; warn if larger than available disk budget.
* SSH port: integer from 1 to 65535; warn for privileged ports other than 22 and for ports already in use.
* Hostname: valid DNS label, no spaces, no leading or trailing hyphen.
* Timezone: must exist under `/usr/share/zoneinfo`.
* Locale: must be available or planned for generation.
* Cloudflare-only HTTP/S: require explicit confirmation that DNS is proxied through Cloudflare.
* Reverse proxy mode: reject combinations that bind multiple services to ports 80/443 without an explicit override.

## 5. Preflight

Show warnings before applying:

```text
Warnings:
- Password SSH login will be disabled after key validation.
- Dokploy requires Docker. Docker has been added automatically.
- Port 80 is already in use by nginx.
- Cloudflare-only HTTP/S can break direct origin access.
```

## 6. Apply

Show live logs:

```text
[RUNNING] Installing Docker
[DONE] Docker installed
[RUNNING] Configuring UFW
[FAILED] Failed to enable UFW
```

## 7. Summary

Show:

* Installed modules
* Failed modules
* Log file path
* Open ports
* Enabled services
* Next commands
* Reboot recommendation

## Screen state model

Every screen should model its content state explicitly:

* `Loading`: async detection, plan generation, or verification is running.
* `Empty`: no matching modules, no warnings, no logs, or no search results.
* `Error`: the screen could not load required data or a recoverable operation failed.
* `Ready`: normal interactive state.

Screens should render these states directly instead of encoding them as nullable data spread across widgets.

## Confirmation dialogs

Use a shared confirmation component in `tui/confirm.rs`.

Confirmations must show:

* Action name
* What will change
* Why confirmation is required
* Rollback or recovery note when available
* Default selection biased toward the safer option

Actions requiring confirmation:

* Apply generated setup plan
* Disable root SSH login
* Disable password SSH login
* Enable UFW
* Change SSH port
* Enable Cloudflare-only HTTP/S
* Overwrite or replace critical config
* Run a remote install script
* Delete, clean up, or remove system resources
* Restart or reload services that may affect active access

---

# Idempotency Rules

The app should be safe to run multiple times.

Each module should:

* Detect whether it is already installed
* Skip completed steps
* Repair partial installs where reasonable
* Avoid overwriting config without backup
* Create timestamped backups before modifying system files
* Show diffs for critical config changes

Critical files to backup before edit:

* `/etc/ssh/sshd_config`
* `/etc/ufw/*`
* `/etc/fail2ban/*`
* `/etc/docker/daemon.json`
* reverse proxy configs
* systemd unit files created by the app

---

# Logging

Log locations:

```text
/var/log/toride/setup.log
/var/log/toride/actions.jsonl
/var/log/toride/report.json
```

For non-root local dry-runs:

```text
~/.local/state/toride/logs/
```

Logs should include:

* Timestamp
* Module name
* Command executed
* Exit code
* stdout/stderr summary
* Duration
* Whether the command changed the system

---

# Safety Rules

Hard rules:

1. Never disable SSH password login before the operator confirms client-side reconnect success.
2. Never disable root login before a sudo user exists and works.
3. Never enable UFW without allowing the active SSH port.
4. Never install conflicting reverse proxies without warning.
5. Never overwrite critical config without backup.
6. Never run remote install scripts silently without showing source and command.
7. Never assume Cloudflare-only mode is safe.
8. Never run destructive cleanup commands without explicit confirmation.

---

# MVP Scope

## v0.1 — buildable in 4–6 weeks

Foundations:

* `cargo-dist` release pipeline producing musl-static x86_64 + aarch64 binaries
* `curl | sh` bootstrap script with SHA256 verification
* Root assertion only for Apply; planning may run without root
* Ratatui async event loop using tokio + crossterm `event-stream` + `CancellationToken`
* Executor with `Action` enum, command spawning, log streaming to TUI via `mpsc`
* Preflight runner: OS detect, systemd present, cloud-init wait, apt-lock flock, RAM/disk check
* JSON + text logging under `/var/log/toride/` (fallback `~/.local/state/toride/` for dry-run as non-root)
* `toride plan --json` for AI-agent and CI consumption

Modules:

* System update / apt baseline packages
* Swap file
* Create sudo user + SSH key + drop-in sshd hardening (with `50-cloud-init` override)
* UFW
* Docker (official repo) + Compose plugin + log rotation + user group
* mise (single module covering Node, Bun, Deno, Go, Rust, Python)

Profiles: Basic and Custom only.

UX: TUI flow (Welcome → Profile → Module selection → Configure → Preflight → Apply → Summary) plus `toride apply --config toride.toml`.

Out of scope for v0.1: Sandbox profile, fail2ban, Cloudflare-only HTTP/S, Tailscale, server managers, reverse proxies, monitoring, backup, unattended-upgrades, sysctl hardening.

## v0.2

* Sandbox profile
* fail2ban with `backend = systemd` baked in for Ubuntu 24.04 / Debian 12
* unattended-upgrades
* Tailscale (pinned version, script URL + SHA shown before exec)
* Cloudflare-only HTTP/S with refresh timer
* Sysctl hardening pack
* Hostname / timezone / locale module
* Dokploy installer
* Coolify installer
* Reverse-proxy modules: Caddy, Traefik, NGINX (native + Docker variants)
* Config export/import

## v0.3

* Cloudflare Tunnel
* WireGuard
* Backup modules: Restic, Borg, Rclone
* Database dump helpers
* Monitoring modules: node_exporter, Uptime Kuma, Netdata, Prometheus, Grafana
* `toride apply --remote user@host` (SSH-out mode)
* RHEL-family support: AlmaLinux, Rocky, Fedora (dnf executor)

## v0.4

* Plugin system (recipes as TOML + signed binaries)
* Team presets / shared profile library
* Server inventory mode (multi-host)
* Optional web dashboard

---

# References

Sources consulted while writing and auditing this plan. Cite when re-researching, updating versions, or onboarding a new contributor.

## Distribution & bootstrap

* cargo-dist releases (musl artifacts) — <https://github.com/axodotdev/cargo-dist/releases>
* cargo-binstall — <https://github.com/cargo-bins/cargo-binstall>
* Rust CLI packaging guide — <https://rust-cli.github.io/book/tutorial/packaging.html>

## Privilege model

* pkexec vs sudo for TTY / raw-mode — <https://gist.github.com/sstavar/d273b6e4a8323b045c2f5b2c95b45c21>
* Rust `sudo` crate (escalate_if_needed) — <https://docs.rs/sudo>

## OS / preflight

* cloud-init dpkg lock contention — <https://github.com/canonical/cloud-init/issues/2908>
* UFW vs nftables on Debian / Ubuntu — <https://betterstack.com/community/guides/linux/ufw-vs-nftables/>
* Ubuntu firewall docs — <https://documentation.ubuntu.com/security/security-features/network/firewall/>

## SSH hardening

* Drop-in `.d` directory practice — <https://ostechnix.com/drop-in-d-directories-linux-configuration-explained/>
* Ubuntu 26 SSH hardening guide — <https://oneuptime.com/blog/post/2026-01-07-ubuntu-ssh-hardening/view>
* Debian 12 SSH hardening — <https://reintech.io/blog/hardening-ssh-server-configuration-debian-12>
* sshaudit hardening guides — <https://www.sshaudit.com/hardening_guides.html>

## fail2ban

* fail2ban systemd backend bug (silent failure on Ubuntu 24.04 / Debian 12) — <https://github.com/fail2ban/fail2ban/issues/3292>
* Install / configure on Ubuntu — <https://linuxcapable.com/how-to-install-fail2ban-on-ubuntu-linux/>

## Docker

* Engine on Debian — <https://docs.docker.com/engine/install/debian/>
* Engine on Ubuntu — <https://docs.docker.com/engine/install/ubuntu/>

## Server managers

* Dokploy installation — <https://docs.dokploy.com/docs/core/installation>
* Coolify installation — <https://coolify.io/docs/get-started/installation>

## Networking

* Tailscale Linux install — <https://tailscale.com/docs/install/linux>
* Tailscale install script — <https://tailscale.com/install.sh>
* Cloudflare IP ranges — <https://www.cloudflare.com/ips/>
* Cloudflare API IPs endpoint — <https://api.cloudflare.com/client/v4/ips>
* Cloudflare IP whitelist UFW 2026 — <https://www.panelica.com/blog/cloudflare-ip-ranges-whitelist-complete-2026-setup-guide-for-nginx-apache-firewalls>

## Language runtime managers

* mise — <https://mise.jdx.dev>
* fnm vs nvm vs Volta 2026 — <https://www.pkgpulse.com/guides/fnm-vs-nvm-vs-volta-nodejs-version-managers-2026>

## TUI / terminal app architecture

* Ratatui v0.30 release highlights — <https://ratatui.rs/highlights/v030/>
* `ratatui::init` — <https://docs.rs/ratatui/latest/ratatui/fn.init.html>
* Ratatui panic hooks — <https://ratatui.rs/recipes/apps/panic-hooks/>
* Ratatui with `color_eyre` — <https://ratatui.rs/recipes/apps/color-eyre/>
* tachyonfx `EffectManager` — <https://docs.rs/tachyonfx/latest/tachyonfx/struct.EffectManager.html>
* tachyonfx DSL — <https://docs.rs/tachyonfx/latest/tachyonfx/dsl/index.html>
* Tokio process command — <https://docs.rs/tokio/latest/tokio/process/struct.Command.html>
* Tokio graceful shutdown — <https://tokio.rs/tokio/topics/shutdown>
* tracing-appender rolling files — <https://docs.rs/tracing-appender/latest/tracing_appender/rolling/>
* Confirmation dialog destructive-action pattern — <https://www.hashbuilds.com/patterns/what-is-confirm-dialog>
* `tui-dialog` announcement — <https://forum.ratatui.rs/t/announcing-tui-dialog/232>
