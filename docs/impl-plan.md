# Implementation Plan — v0.2 → v0.4

Extends existing v0.1 MVP. Follows `design.md` architecture and `e2e-testing.md` conventions.

---

## Phase 2: v0.2 Modules & Features

### 2.1 New modules (src/modules/)

Each module: zero-sized struct, implements `SetupModule`, registered in `registry()`.

**fail2ban.rs**
- `id()` → `ModuleId::Fail2Ban`
- `dependencies()` → `[ModuleId::SystemUpdate]`
- `category()` → `Category::FirewallAndSecurity`
- `plan()`: apt install fail2ban python3-systemd, write `/etc/fail2ban/jail.local` with `backend = systemd` and `[sshd] enabled = true`, systemctl enable/start
- `preflight()`: check if fail2ban already installed
- `verify()`: check `fail2ban-client status`

**unattended_upgrades.rs**
- `id()` → `ModuleId::UnattendedUpgrades`
- `dependencies()` → `[ModuleId::SystemUpdate]`
- `category()` → `Category::SystemBasics`
- `plan()`: apt install unattended-upgrades apt-listchanges, write `/etc/apt/apt.conf.d/50unattended-upgrades` config, dpkg-reconfigure
- `verify()`: check unattended-upgrades active

**tailscale.rs**
- `id()` → `ModuleId::Tailscale`
- `dependencies()` → `[]`
- `category()` → `Category::Networking`
- `plan()`: DownloadScript for install.sh with sha256, systemctl enable/start, `tailscale up`
- `preflight()`: check if tailscale already installed
- `verify()`: check `tailscale status`

**cloudflare_http.rs**
- `id()` → `ModuleId::CloudflareHttp`
- `dependencies()` → `[ModuleId::Ufw]`
- `category()` → `Category::FirewallAndSecurity`
- `plan()`: fetch IPs from `https://www.cloudflare.com/ips-v4` and `ips-v6`, cache to `/var/lib/toride/cloudflare-ips.txt`, add UFW rules for each range on 80/443
- `verify()`: check UFW rules contain cloudflare ranges

**sysctl.rs**
- `id()` → `ModuleId::SysctlHardening`
- `dependencies()` → `[]`
- `category()` → `Category::FirewallAndSecurity`
- `plan()`: WriteFile `/etc/sysctl.d/99-toride.conf` with kernel hardening values, `sysctl --system`
- `verify()`: check sysctl values applied

**hostname.rs**
- `id()` → `ModuleId::Hostname`
- `dependencies()` → `[]`
- `category()` → `Category::SystemBasics`
- `plan()`: Exec hostnamectl set-hostname, WriteFile /etc/hosts update
- `verify()`: check hostname matches

**timezone.rs**
- `id()` → `ModuleId::Timezone`
- `dependencies()` → `[]`
- `category()` → `Category::SystemBasics`
- `plan()`: Exec timedatectl set-timezone, symlink /etc/localtime
- `verify()`: check timedatectl output

**dokploy.rs**
- `id()` → `ModuleId::Dokploy`
- `dependencies()` → `[ModuleId::Docker]`
- `conflicts()` → `[ModuleId::Coolify]`
- `category()` → `Category::ServerManagers`
- `plan()`: Exec docker run dokploy/dokploy with volume mounts
- `verify()`: check docker container running

**coolify.rs**
- `id()` → `ModuleId::Coolify`
- `dependencies()` → `[ModuleId::Docker]`
- `conflicts()` → `[ModuleId::Dokploy]`
- `category()` → `Category::ServerManagers`
- `plan()`: DownloadScript coolify install script, Exec to configure
- `verify()`: check docker container running

**caddy.rs**
- `id()` → `ModuleId::Caddy`
- `dependencies()` → `[]`
- `conflicts()` → `[ModuleId::Nginx, ModuleId::Traefik]`
- `category()` → `Category::ReverseProxy`
- `plan()`: apt install caddy (or DownloadScript), WriteFile Caddyfile, systemctl enable/start
- `verify()`: check caddy binary

**nginx.rs**
- `id()` → `ModuleId::Nginx`
- `dependencies()` → `[]`
- `conflicts()` → `[ModuleId::Caddy, ModuleId::Traefik]`
- `category()` → `Category::ReverseProxy`
- `plan()`: apt install nginx, WriteFile config, systemctl enable/start
- `verify()`: check nginx binary

**traefik.rs**
- `id()` → `ModuleId::Traefik`
- `dependencies()` → `[]`
- `conflicts()` → `[ModuleId::Caddy, ModuleId::Nginx]`
- `category()` → `Category::ReverseProxy`
- `plan()`: DownloadScript traefik binary, WriteFile config, WriteFile systemd unit, systemctl enable/start
- `verify()`: check traefik binary

### 2.2 Model changes (src/tui/model.rs)

New `ModuleId` variants:
```
Fail2Ban, UnattendedUpgrades, Tailscale, CloudflareHttp,
SysctlHardening, Hostname, Timezone, Dokploy, Coolify,
Caddy, Nginx, Traefik
```

New `Category` variants:
```
Networking, ServerManagers, ReverseProxy
```

New `Profile` variant: `Sandbox`

New `FormField` variants:
```
Hostname, Timezone
```

New `PaletteCmd` variants:
```
ExportJson, ExportToml
```

### 2.3 Profiles

**src/profiles/sandbox.rs** — new file:
- Modules: SystemUpdate, Swap, UserSsh, Ufw, Docker, Mise
- Less strict SSH (root login stays, password stays)

**src/profiles/mod.rs** — add Sandbox branch

**src/profiles/basic.rs** — add Fail2Ban, UnattendedUpgrades

### 2.4 Executor changes

**src/executor/command.rs** — no new InstallAction variants needed for v0.2

**src/executor/plan.rs** — update `generate_preflight_warnings()` for:
- Dokploy/Coolify requires Docker
- Conflicting reverse proxies
- Cloudflare-only without Cloudflare DNS

### 2.5 Config changes (src/config/schema.rs)

Add sections:
```toml
[security]
fail2ban = true
cloudflare_only_http = false
auto_security_updates = true

[server_manager]
manager = "none"  # none | dokploy | coolify

[reverse_proxy]
mode = "none"     # none | caddy | nginx | traefik | managed

[networking]
tailscale = false
```

### 2.6 TUI update/view changes

- Profile select screen: add Sandbox option
- Module select: handle new categories (Networking, ServerManagers, ReverseProxy)
- Configure screen: add Hostname, Timezone fields
- Preflight warnings for new conflict/dependency combinations
- Apply screen: no changes needed (generic)

### 2.7 E2E tests (tests/e2e/)

New test files:
- `tests/e2e/sandbox_profile.rs` — Sandbox profile flow
- `tests/e2e/conflicts.rs` — conflicting module selection warnings

New tests in existing files:
- `profiles.rs`: sandbox_profile_shows_runtimes
- `overlays.rs`: palette_export_command

### 2.8 Unit tests (tests/unit_tests.rs)

New tests:
- fail2ban plan generates correct actions
- tailscale plan includes sha256 verification
- cloudflare_http depends on ufw
- dokploy depends on docker
- coolify conflicts with dokploy
- reverse proxy mutual exclusion
- sandbox profile has correct modules
- unattended_upgrades plan generates config
- sysctl hardening values are valid

---

## Phase 3: v0.3 Modules & Features

### 3.1 New modules

**cloudflare_tunnel.rs**
- `id()` → `ModuleId::CloudflareTunnel`
- `dependencies()` → `[]`
- `category()` → `Category::Networking`
- `plan()`: DownloadScript cloudflared, Exec `cloudflared tunnel login`, WriteFile config, systemd unit
- `verify()`: check cloudflared binary

**wireguard.rs**
- `id()` → `ModuleId::Wireguard`
- `dependencies()` → `[]`
- `category()` → `Category::Networking`
- `plan()`: apt install wireguard, Exec wg genkey/genpubkey, WriteFile wg0.conf, systemctl enable/start
- `verify()`: check wg interface

**restic.rs**
- `id()` → `ModuleId::Restic`
- `dependencies()` → `[]`
- `category()` → `Category::Backup`
- `plan()`: apt install restic, WriteFile backup script, systemd timer
- `verify()`: check restic binary

**borg.rs**
- `id()` → `ModuleId::Borg`
- `dependencies()` → `[]`
- `category()` → `Category::Backup`
- `plan()`: apt install borgbackup, WriteFile config
- `verify()`: check borg binary

**rclone.rs**
- `id()` → `ModuleId::Rclone`
- `dependencies()` → `[]`
- `category()` → `Category::Backup`
- `plan()`: DownloadScript rclone install, Exec rclone config
- `verify()`: check rclone binary

**node_exporter.rs**
- `id()` → `ModuleId::NodeExporter`
- `dependencies()` → `[]`
- `category()` → `Category::Monitoring`
- `plan()`: DownloadScript node_exporter, WriteFile systemd unit, systemctl enable/start
- `verify()`: check node_exporter running

**uptime_kuma.rs**
- `id()` → `ModuleId::UptimeKuma`
- `dependencies()` → `[ModuleId::Docker]`
- `category()` → `Category::Monitoring`
- `plan()`: Exec docker run uptime-kuma container
- `verify()`: check container running

**netdata.rs**
- `id()` → `ModuleId::Netdata`
- `dependencies()` → `[]`
- `category()` → `Category::Monitoring`
- `plan()`: DownloadScript netdata kickstart, systemctl enable/start
- `verify()`: check netdata service

### 3.2 New InstallAction variant

```rust
DnfInstall { packages: Vec<String> }       // RHEL-family support
DnfRepoAdd { name: String, baseurl: String, gpgkey: String }
```

### 3.3 Model changes

New `ModuleId` variants:
```
CloudflareTunnel, Wireguard, Restic, Borg, Rclone,
NodeExporter, UptimeKuma, Netdata
```

New `Category` variants:
```
Backup, Monitoring
```

### 3.4 Executor changes

**src/executor/command.rs** — handle `DnfInstall`, `DnfRepoAdd`
**src/system/os_detect.rs** — add `is_rhel_family()` helper
**src/modules/mod.rs** — new `InstallAction` variants

### 3.5 CLI changes (src/main.rs)

```rust
Commands::Apply {
    #[arg(long)]
    remote: Option<String>,  // user@host for SSH-out mode
}
```

New `src/executor/remote.rs` — SSH-out executor:
- Connect via SSH to remote host
- Transfer binary
- Execute toride apply remotely
- Stream progress back

### 3.6 E2E tests

- `tests/e2e/monitoring.rs` — uptime kuma requires docker
- `tests/e2e/backup.rs` — backup module selection

### 3.7 Unit tests

- RHEL detection logic
- DNF action to_shell_preview
- backup module plan generation
- monitoring module dependencies

---

## Phase 4: v0.4 Features

### 4.1 Plugin system

**src/plugins/mod.rs** — new module:
```rust
trait ToridePlugin: Send + Sync {
    fn id(&self) -> &str;
    fn version(&self) -> &str;
    fn modules(&self) -> Vec<Box<dyn SetupModule>>;
}
```

**src/plugins/loader.rs** — load plugins from:
- `/etc/toride/plugins/`
- `~/.config/toride/plugins/`
- Parse TOML recipe files

**src/plugins/recipe.rs** — TOML recipe format:
```toml
[recipe]
name = "custom-stack"
version = "1.0"

[module.install]
steps = [
    { type = "apt", packages = ["foo"] },
    { type = "exec", cmd = "foo-init" },
]
```

### 4.2 Team presets

**src/presets/mod.rs** — fetch shared profiles from URL or file
- `toride preset list`
- `toride preset apply <name>`

### 4.3 Server inventory

**src/inventory/mod.rs** — multi-host management:
- SSH connection pool
- Parallel execution across hosts
- Status dashboard

### 4.4 Model changes

New `Screen` variants for inventory/dashboard
New `Category::Plugins`
Plugin-related PaletteCmd variants

### 4.5 Web dashboard (optional)

Feature-gated behind `--features web`:
- `axum` for HTTP server
- WebSocket for real-time status
- Serve static assets

---

## Implementation Order

### Sprint 1: v0.2 foundation (models + categories + profiles)
1. Add new ModuleId, Category, Profile, FormField variants to model.rs
2. Add new Category labels and ordering
3. Create src/profiles/sandbox.rs
4. Update registry() in modules/mod.rs
5. Update config schema
6. Unit tests for new types

### Sprint 2: v0.2 modules batch 1 (security + system)
7. fail2ban.rs
8. unattended_upgrades.rs
9. sysctl.rs
10. hostname.rs
11. timezone.rs

### Sprint 3: v0.2 modules batch 2 (networking + servers)
12. tailscale.rs
13. cloudflare_http.rs
14. dokploy.rs
15. coolify.rs

### Sprint 4: v0.2 modules batch 3 (reverse proxy)
16. caddy.rs
17. nginx.rs
18. traefik.rs

### Sprint 5: v0.2 TUI + integration
19. Profile select: add Sandbox
20. Module select: new categories
21. Configure screen: new fields
22. Preflight warnings for new combinations
23. Config export/import via palette

### Sprint 6: v0.2 tests
24. E2E: sandbox profile flow
25. E2E: conflict warnings
26. Unit: all new module plans
27. Unit: dependency/conflict resolution
28. Unit: sandbox profile defaults

### Sprint 7: v0.3 foundation
29. New ModuleId/Category variants
30. New InstallAction variants (DnfInstall, DnfRepoAdd)
31. RHEL detection in os_detect.rs
32. DNF handler in command.rs

### Sprint 8: v0.3 modules batch 1
33. cloudflare_tunnel.rs
34. wireguard.rs
35. restic.rs
36. borg.rs
37. rclone.rs

### Sprint 9: v0.3 modules batch 2
38. node_exporter.rs
39. uptime_kuma.rs
40. netdata.rs

### Sprint 10: v0.3 CLI + remote
41. SSH-out mode (src/executor/remote.rs)
42. CLI --remote flag
43. Remote progress streaming

### Sprint 11: v0.3 tests
44. E2E: monitoring requires docker
45. E2E: backup module selection
46. Unit: DNF actions
47. Unit: all v0.3 module plans

### Sprint 12: v0.4 plugin system
48. Plugin trait and loader
49. TOML recipe parser
50. Plugin registration in registry

### Sprint 13: v0.4 presets + inventory
51. Team preset fetch/apply
52. Multi-host inventory
53. Parallel execution

### Sprint 14: v0.4 tests + polish
54. Plugin loading tests
55. Inventory integration tests
56. Full regression suite
57. Audit against plan.md
