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

* Node/NVM
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
* Node/NVM
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
│  ├─ NVM
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
[ ] NVM
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

Toride consolidates Node, Bun, Deno, Go, Rust, and Python under a single runtime manager rather than installing each via its own bespoke script. Preferred manager: **mise** (`https://mise.jdx.dev`). It is a single static binary, handles all six languages, and avoids the `.bashrc` mutation that NVM-style installers depend on.

Options:

* mise (recommended)
* asdf (legacy compatibility)
* Per-language scripts (NodeSource, rustup, Go tarball, etc.) — fallback only

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
* Warn if `/usr/bin/node` from apt is present — coexisting versions confuse `which node`.
* If Bun is selected, still ask whether Node compatibility is needed.
* `rustup` may be used in place of mise for Rust if the user prefers — mise's Rust handling is thinner than rustup's.

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
8. Verify key login works: spawn `ssh -o BatchMode=yes -o StrictHostKeyChecking=no -i <key> <user>@127.0.0.1 true` and assert exit 0. Fail closed.
9. Write hardening to `/etc/ssh/sshd_config.d/00-toride.conf` (never edit the main `sshd_config` directly — both Debian 12 and Ubuntu 24.04 ship the `Include` line).
10. Detect `50-cloud-init.conf`. If it sets `PasswordAuthentication yes`, override it by removing the file or commenting that line — OpenSSH uses the **first** value found, so a later drop-in alone is not sufficient.
11. Only then offer to disable root login (`PermitRootLogin no`).
12. Only then offer to disable password login (`PasswordAuthentication no`, `KbdInteractiveAuthentication no`).
13. Validate sshd config using `sshd -t` before applying.
14. Reload SSH via `systemctl reload ssh` (reload, not restart — keeps the active session alive).
15. Show emergency rollback command (path to backup + `systemctl reload ssh`).

The app must never blindly disable root/password login before verifying key access.

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

Toride requires root. The binary asserts `EUID == 0` at startup and exits with a clear error otherwise. It never spawns `sudo` per-command — sudo prompts corrupt Ratatui raw-mode input and produce inconsistent state across modules.

Recommended invocation:

```bash
sudo -E toride
```

The `sudo` crate's `escalate_if_needed()` may re-exec under sudo before the terminal enters raw mode, never after. `pkexec` is not a fit (headless VPS has no Polkit agent).

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
* `tokio` (full features) for async event loop and process spawning
* `tokio-util` for `CancellationToken`
* `futures` for `FutureExt` / `StreamExt`
* `clap` (derive) for CLI flags
* `serde` / `serde_json` / `toml` for config
* `color-eyre` for app-level errors and panic handler
* `thiserror` for typed errors
* `tracing` for logs
* `tracing-subscriber` with `env-filter` and `json` features
* `reqwest` (rustls backend) for downloading install scripts / IP ranges
* `sha2` / `hex` for verifying install-script and binary checksums
* `which` for binary detection
* `nix` for Unix syscalls (euid, file modes, signals)
* `dirs` for config paths
* `sudo` for `escalate_if_needed()` at startup
* `async-trait` for the module trait

## Internal modules

```text
src/
├─ main.rs
├─ app.rs
├─ tui/
│  ├─ mod.rs
│  ├─ screens.rs
│  ├─ widgets.rs
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
node_method = "nvm"
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
* Node install method
* Reverse proxy mode
* Server manager choice
* Cloudflare-only HTTP/S confirmation

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

1. Never disable SSH password login before confirming key login.
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
* Root assertion at startup; no sudo elevation inside raw mode
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