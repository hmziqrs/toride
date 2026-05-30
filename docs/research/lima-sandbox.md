# Lima Sandbox Runbook

This document defines how Toride should use Lima for destructive Linux testing.

Toride is a guided VPS setup tool. Its apply path will intentionally mutate a machine: package manager state, users, groups, SSH daemon configuration, firewall rules, Docker, services, files under `/etc`, and sometimes reboot-required state. The sandbox must therefore be a real Linux VM that can be destroyed or restored quickly.

Lima is the preferred local sandbox runner because it is CLI-first, lightweight on macOS, supports multiple Linux distributions, provides SSH access, file sharing, port forwarding, and snapshot commands, and can be driven by an AI agent without a GUI.

Audit status:

* Lima is not installed in the current development workspace, so local command execution could not be validated here.
* Commands, flags, template names, mounts, VZ behavior, `copy`, `validate`, and snapshot syntax in this document were cross-checked against Lima's current public docs on 2026-05-14.
* Network/firewall realism requirements were incorporated on 2026-05-30 from a follow-up Lima sandbox review.
* Lima snapshots are documented. Scripts must still support a delete-and-recreate fallback because snapshot behavior can vary by Lima version, guest image, and VM driver.
* The sandbox flow is workable for the current repo state, but Toride itself is not implemented yet beyond `src/main.rs`. The destructive apply examples describe the intended CLI once Toride has apply/dry-run commands.
* The biggest project-specific risk is SSH/firewall testing: Lima uses SSH for `limactl shell` and `limactl copy`, so a Toride run that breaks guest SSH may also break artifact collection.

---

# Goals

The Lima sandbox must support:

* Repeated destructive Toride runs without damaging the host.
* Fast reset back to a known-clean baseline.
* Multiple target distributions.
* Real `systemd`, real package managers, real users, real services, and real network behavior.
* Host-to-guest binary transfer for locally built Toride artifacts.
* Agent-friendly commands that can be scripted without opening a GUI.
* Clear teardown when a VM is corrupted beyond repair.

The sandbox is not a substitute for unit tests, dry-run tests, or PTY E2E tests. It is the destructive integration layer.

---

# Recommended Distro Matrix

Toride should keep a small but representative matrix.

## Primary

These should be tested before trusting a release:

* Ubuntu 24.04 LTS
* Ubuntu 26.04 LTS
* Debian 12
* Debian 13

## Secondary

These are important for later compatibility work:

* Rocky Linux 9
* Rocky Linux 10

## Optional Later

Add only after the primary matrix is reliable:

* AlmaLinux 9 / 10
* Fedora Server
* openSUSE Leap
* Arch Linux

---

# Why Not Docker

Docker containers are useful for narrow tests, but they are not the main sandbox for Toride.

Containers do not reliably match VPS behavior for:

* `systemd`
* SSH daemon lifecycle
* UFW and nftables behavior
* Docker installation and Docker-in-Docker
* kernel modules and cgroups
* reboot-required state
* service enablement
* cloud-init behavior
* package lock timing on fresh boot

Use containers only for command rendering, package detection, and dry-run checks. Use Lima VMs for destructive apply tests.

---

# Network Test Modes

Lima is a real Linux VM layer, but its networking is not one universal model. Firewall tests must state which network mode they are validating, otherwise Toride can get false confidence from tests that only exercise localhost, forwarded ports, or traffic originating inside the same guest.

## Mode A: Default Lima Networking

Use default Lima networking for:

* package installation
* SSH daemon configuration syntax checks
* user, group, and sudo checks
* service enablement
* Docker installation
* non-firewall dry-run and apply smoke tests

This mode is enough to prove that Toride can mutate real Linux state. It is not enough to prove that ports are blocked from another machine.

## Mode B: Host-To-Guest SSH Validation

Use Lima's generated SSH config for direct reconnect tests from the host:

```bash
ssh -F ~/.lima/toride-u2404/ssh.config lima-toride-u2404 true
```

Run this before and after modules that touch users, sudo, SSH daemon configuration, UFW, nftables, Fail2Ban, or reboot behavior.

This mode proves that the control path still works and that key-based login survives. It does not model public internet traffic.

## Mode C: Host-To-Guest VM IP Checks

Use this when a test needs to address a guest IP rather than a Lima port forward. The script must make the network mode explicit through the Lima template or `limactl start --network ...`, then record the target IP in the artifacts.

Use this mode for:

* checking services bound to the guest address
* distinguishing localhost-only services from guest-reachable services
* validating host-originated access to an explicitly opened port

Do not treat this as a replacement for an attacker VM. The traffic source is still the host-side Lima network path.

## Mode D: Guest-To-Guest Attacker VM

Use a second Lima VM on the same named network when testing firewall allow/block behavior. The attacker VM should not run Toride and should stay disposable.

Recommended names:

```text
toride-netprobe
toride-u2404-attacker
```

The probe VM should run checks such as:

```bash
limactl shell toride-netprobe -- nc -vz <target-ip> 22
limactl shell toride-netprobe -- nc -vz -w 3 <target-ip> 8080
limactl shell toride-netprobe -- curl -fsS http://<target-ip>:8080/health
```

Use this mode for:

* proving SSH is allowed from outside the target guest
* proving a closed test port is blocked
* proving explicitly opened application ports are reachable
* checking IPv4 and IPv6 separately when the shared network provides both

## Mode E: Real VPS Canary

Use a disposable real VPS before release for behavior Lima cannot faithfully reproduce:

* public IPv4 and IPv6 exposure
* provider firewall interaction
* provider base images and metadata services
* Cloudflare-only allowlists
* reboot persistence on actual cloud boot paths
* rescue console and recovery expectations

The Lima lane remains the default local destructive loop. The VPS lane is the final canary, not the primary development loop.

---

# Host Requirements

Assumed host:

* macOS
* Homebrew installed
* Enough disk for multiple VM disks
* Apple Silicon or Intel Mac

Install Lima:

```bash
brew install lima
```

Verify:

```bash
limactl --version
limactl list
limactl start --list-templates
```

The scripts should require Lima 2.x or newer for this plan. If `ubuntu-26.04`, `debian-13`, `rocky-10`, `--mount-none`, or `snapshot` are missing, upgrade Lima before continuing.

On macOS, prefer Lima's native virtualization backend where possible:

```yaml
vmType: vz
```

If a guest image fails under `vz`, fall back to QEMU for that distro:

```yaml
vmType: qemu
```

For automation, prefer non-interactive Lima commands:

```bash
limactl start --tty=false ...
limactl create --tty=false ...
```

`--tty=false` disables Lima's TUI/editor prompts. Prefer it in scripts because it is explicit. Current Lima command references also expose `-y` as an alias for `--tty=false`; avoid relying on shorthand in long-lived automation.

---

# Directory Layout

Use a repo-local sandbox directory for templates and agent scripts:

```text
dev/
`-- sandbox/
    `-- lima/
        |-- README.md
        |-- images/
        |   |-- ubuntu-24.04/
        |   |-- ubuntu-26.04/
        |   |-- debian-12/
        |   |-- debian-13/
        |   |-- rocky-9/
        |   `-- rocky-10/
        |-- templates/
        |   |-- ubuntu-24.04.yaml
        |   |-- ubuntu-26.04.yaml
        |   |-- debian-12.yaml
        |   |-- debian-13.yaml
        |   |-- rocky-9.yaml
        |   `-- rocky-10.yaml
        `-- scripts/
            |-- create.sh
            |-- reset.sh
            |-- run.sh
            |-- netprobe.sh
            |-- destroy.sh
            `-- matrix.sh
```

The user may provide initial images under `dev/sandbox/lima/images/<distro>/`. The agent should use those images when present. If no local image is present, the agent may use a Lima built-in template or an official cloud image URL.

Do not commit large VM image files. Add this ignore rule when the directory is created:

```gitignore
dev/sandbox/lima/images/**/*.qcow2
dev/sandbox/lima/images/**/*.img
dev/sandbox/lima/images/**/*.raw
dev/sandbox/lima/images/**/*.iso
```

---

# Image Input Contract

The AI agent expects each supplied image directory to contain:

```text
dev/sandbox/lima/images/<distro>/
|-- image-aarch64.qcow2
|-- image-x86_64.qcow2
`-- SHA256SUMS
```

Preferred image format:

* `qcow2` cloud image with cloud-init support

Accepted with caveats:

* `.img` cloud image
* `.raw` disk image

Do not use ISO installer images for the normal test loop. Lima distribution templates are built around cloud-style disk images. ISO installation is slower, less reproducible, harder for an agent to automate, and should be treated as a separate manual image-building task.

The `SHA256SUMS` file should contain a hash for the image:

```text
<sha256>  image-aarch64.qcow2
<sha256>  image-x86_64.qcow2
```

Verify before use:

```bash
cd dev/sandbox/lima/images/ubuntu-24.04
shasum -a 256 -c SHA256SUMS
```

If only one architecture is provided, the template for that distro must set `arch` to that architecture and must not pretend the same image supports both `aarch64` and `x86_64`.

For Apple Silicon Macs, use `aarch64` images by default. Use `x86_64` only when testing x86-specific behavior, and expect it to be slower because it requires emulation. For Intel Macs, use `x86_64`.

Cross-architecture testing on Apple Silicon should use QEMU:

```yaml
vmType: qemu
arch: x86_64
```

Native-architecture testing should use `vz` where possible.

---

# Instance Naming

Use stable names:

```text
toride-u2404
toride-u2604
toride-d12
toride-d13
toride-r9
toride-r10
```

For attacker/probe VMs, keep names distinct from target VMs:

```text
toride-netprobe
toride-u2404-attacker
toride-d13-attacker
```

Do not use random instance names in scripts. Stable names make cleanup and agent recovery simpler.

---

# Baseline Lifecycle

Every distro follows the same lifecycle:

1. Create VM from template.
2. Boot VM.
3. Wait for cloud-init and package manager locks.
4. Install only baseline tools needed for testing.
5. Stop VM.
6. Create `clean` snapshot.
7. For each destructive test, restore `clean`, start, run Toride, collect logs.
8. Destroy and recreate if restore cannot recover the VM.

The important invariant:

> Toride apply tests always start from the `clean` snapshot.

Do not install Toride or developer build tools into the destructive test VM before creating `clean`. Build tools belong on the host or in a separate builder VM.

---

# Generic Lima Template

Use this as the baseline shape for templates.

```yaml
# dev/sandbox/lima/templates/ubuntu-24.04.yaml
minimumLimaVersion: "2.0.0"
vmType: vz
arch: default
cpus: 2
memory: 4GiB
disk: 24GiB

images:
  - location: "/ABS/PATH/TO/dev/sandbox/lima/images/ubuntu-24.04/image-aarch64.qcow2"
    arch: "aarch64"
  - location: "/ABS/PATH/TO/dev/sandbox/lima/images/ubuntu-24.04/image-x86_64.qcow2"
    arch: "x86_64"

mounts: []

containerd:
  system: false
  user: false

provision:
  - mode: system
    script: |
      set -eux
      if command -v cloud-init >/dev/null 2>&1; then
        cloud-init status --wait || true
      fi
      if command -v apt-get >/dev/null 2>&1; then
        export DEBIAN_FRONTEND=noninteractive
        apt-get update
        apt-get install -y ca-certificates curl sudo openssh-server systemd systemd-sysv iproute2
      elif command -v dnf >/dev/null 2>&1; then
        dnf install -y ca-certificates curl sudo openssh-server systemd iproute
      fi
      systemctl enable ssh || systemctl enable sshd || true
      systemctl start ssh || systemctl start sshd || true
```

Notes:

* Host mounts are disabled by default. This is intentional: destructive guest commands should not receive direct filesystem access to the host repo.
* Copy test binaries into `/tmp` or `/opt/toride-test` inside the guest before running them.
* Use absolute local image paths in generated YAML. This avoids ambiguity about whether Lima resolves relative paths from the repo root, current shell directory, or template location.
* Use `4GiB` memory by default. Increase to `6GiB` or `8GiB` for Ubuntu 26.04 if heavy package flows become slow.
* For Rocky 10, verify the image supports the host CPU architecture. Rocky 10 changed some architecture baselines on x86.
* If a distro only works under QEMU, change `vmType` to `qemu`. Do not use QEMU `9p` mounts with Rocky/Alma-style guests; Lima documents 9p incompatibility with those kernels.

---

# Built-In Template Fallback

If no user-supplied image exists, use Lima's templates where available:

```bash
limactl start --tty=false --name=toride-u2404 --mount-none template:ubuntu-24.04
limactl start --tty=false --name=toride-u2604 --mount-none template:ubuntu-26.04
limactl start --tty=false --name=toride-d12 --mount-none template:debian-12
limactl start --tty=false --name=toride-d13 --mount-none template:debian-13
limactl start --tty=false --name=toride-r9 --mount-none template:rocky-9
limactl start --tty=false --name=toride-r10 --mount-none template:rocky-10
```

Template names can change by Lima version. The agent must check available templates before assuming a name:

```bash
limactl start --list-templates
```

If the exact distro template is missing, use a custom YAML with an official cloud image URL or ask the user for the image.

Do not use `template:ubuntu` for the Ubuntu LTS test lane. Lima's `ubuntu` alias may point at the newest interim release, not the LTS version Toride wants to validate.

Do not use `template:default` for destructive Toride tests. It is convenient for general Lima use, but it includes Lima's default container tooling and defaults that are not as close to a fresh VPS as the versioned distro templates.

---

# Creating a VM

From the repo root:

```bash
limactl create --tty=false --name=toride-u2404 --mount-none dev/sandbox/lima/templates/ubuntu-24.04.yaml
limactl start toride-u2404
```

Check status:

```bash
limactl list
limactl shell toride-u2404 uname -a
limactl shell toride-u2404 cat /etc/os-release
```

Check `systemd`:

```bash
limactl shell toride-u2404 systemctl is-system-running
```

`degraded` is acceptable immediately after boot if the failed unit is irrelevant to the test. `offline` or `Failed to connect to bus` is not acceptable for Toride integration testing.

---

# Creating the Clean Snapshot

After the baseline setup finishes:

```bash
limactl stop toride-u2404
limactl snapshot create toride-u2404 --tag clean
limactl snapshot list toride-u2404
```

The `clean` snapshot should represent a fresh VPS-like state:

* package metadata updated once
* SSH server installed and running
* sudo available
* no Toride changes applied
* no Docker installed unless the base cloud image already includes it
* no UFW changes unless the base cloud image already includes them
* no extra users except image defaults

Do not create a snapshot after running Toride unless it is explicitly named for debugging.

---

# Resetting Before Every Test

Before every destructive apply test:

```bash
limactl stop toride-u2404 || true
limactl snapshot apply toride-u2404 --tag clean
limactl start toride-u2404
```

Then verify the guest identity:

```bash
limactl shell toride-u2404 cat /etc/os-release
limactl shell toride-u2404 id
limactl shell toride-u2404 systemctl is-system-running || true
```

The agent should treat failed snapshot restore as a reason to destroy and recreate the instance.

Even when snapshots are available, every script must support this slower fallback:

```bash
limactl stop toride-u2404 || true
limactl delete -f toride-u2404
limactl create --tty=false --name=toride-u2404 --mount-none dev/sandbox/lima/templates/ubuntu-24.04.yaml
limactl start toride-u2404
limactl stop toride-u2404
limactl snapshot create toride-u2404 --tag clean
limactl start toride-u2404
```

If snapshots prove unreliable for a given Lima version or VM driver, delete-and-recreate becomes the canonical reset path for that lane.

When using delete-and-recreate as the reset path, scripts should cache downloaded images through Lima's normal cache and avoid deleting user-supplied images. The VM disk is disposable; the source image is not.

---

# Snapshots Are Not Backups

For Toride testing, a Lima snapshot is a fast local reset point, not a durable backup.

Keep these durable inputs instead:

* the original user-supplied cloud image
* the image checksum file
* the generated Lima template
* sandbox scripts
* collected test artifacts

If a VM becomes valuable for debugging, export evidence from it before deleting it:

```bash
mkdir -p .sandbox-artifacts/toride-u2404/debug
limactl shell toride-u2404 journalctl -b --no-pager > .sandbox-artifacts/toride-u2404/debug/journal.txt
limactl shell toride-u2404 systemctl --failed --no-pager > .sandbox-artifacts/toride-u2404/debug/systemd-failed.txt
limactl shell toride-u2404 cat /etc/os-release > .sandbox-artifacts/toride-u2404/debug/os-release.txt
```

Do not depend on snapshots to move state between machines. Recreate VMs from images and templates instead.

---

# Running Toride in the Guest

Do not copy `target/release/toride` from a macOS host into the Linux guest. That binary is a macOS binary and will not execute in Linux.

Recommended build path: produce static Linux binaries on the host, matching the guest architecture. This aligns with Toride's release plan for `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl`.

Install the cross-build tooling once on the host:

```bash
brew install zig
cargo install cargo-zigbuild
rustup target add aarch64-unknown-linux-musl
rustup target add x86_64-unknown-linux-musl
```

Build both Linux binaries:

```bash
cargo zigbuild --release --target aarch64-unknown-linux-musl
cargo zigbuild --release --target x86_64-unknown-linux-musl
```

Select the binary that matches the guest:

```bash
limactl shell toride-u2404 uname -m
```

Mapping:

```text
aarch64 -> target/aarch64-unknown-linux-musl/release/toride
x86_64  -> target/x86_64-unknown-linux-musl/release/toride
```

Copy into the guest:

```bash
limactl copy ./target/aarch64-unknown-linux-musl/release/toride toride-u2404:/tmp/toride
limactl shell toride-u2404 chmod +x /tmp/toride
```

Verify it is executable in Linux:

```bash
limactl shell toride-u2404 /tmp/toride --version || limactl shell toride-u2404 /tmp/toride
```

Run dry-run first:

```bash
limactl shell toride-u2404 /tmp/toride --dry-run
```

Run apply only after dry-run is sane:

```bash
limactl shell toride-u2404 sudo /tmp/toride apply --profile sandbox
```

If Toride is interactive-only at that point in development, run an interactive shell:

```bash
limactl shell toride-u2404
sudo /tmp/toride
```

The final scripted CLI shape may change as Toride evolves. The sandbox contract should remain stable: reset, copy binary, run dry-run, run apply, collect logs.

Current repo reality: as of this document, `src/main.rs` only prints `Hello, world!`. The sandbox can already validate that a Linux binary transfers and executes, but it cannot validate destructive Toride behavior until the executor and CLI commands exist.

Fallback build path: if host cross-compilation is blocked, build inside a separate non-destructive builder VM, not inside the clean destructive test VM. The destructive VM should start as close to a fresh VPS as possible before Toride runs.

---

# Required Destructive Test Cases

The sandbox lifecycle is only useful if it proves concrete Toride behavior. Each primary distro should eventually run this case matrix from a clean snapshot.

Required cases:

* Dry-run makes no filesystem, package, service, user, sudo, SSH, or firewall changes.
* Apply is idempotent: the first run may change state; the second run should report no required changes.
* SSH password login is disabled while key login still works.
* Root SSH login is disabled.
* A newly created sudo user can reconnect and run `sudo -n true` when configured for passwordless sudo, or can run sudo with the expected password flow when passwordless sudo is not configured.
* UFW or nftables allows SSH before the firewall is enabled.
* Firewall rules block a known closed test port from the attacker VM.
* Firewall rules allow explicitly opened ports from the attacker VM.
* Fail2Ban installs, starts, and creates the expected jail state when that module is enabled.
* Docker installs, starts, is enabled, and survives reboot when that module is enabled.
* Reboot after apply preserves SSH access.
* Toride prints a rollback or recovery message before risky SSH and firewall steps.

Each case should collect enough evidence to debug a failure without manually logging into the VM later: Toride output, journal, service status, relevant config files, firewall state, and SSH reconnect output.

---

# Syntax Validation Gates

Toride should validate dangerous configuration before activating it. A sandbox test should fail if Toride writes invalid config or skips a required validation gate.

Recommended gates:

```bash
sudo sshd -t
sudo visudo -cf /etc/sudoers
sudo visudo -cf /etc/sudoers.d/<toride-file>
sudo nft -c -f <ruleset-file>
sudo ufw --dry-run enable
```

Notes:

* `sshd -t` may require the absolute path `/usr/sbin/sshd` on some distros.
* `ufw --dry-run enable` is useful where available, but nftables syntax checks are still needed when Toride writes nftables rules directly.
* Syntax validation is not a substitute for reconnect tests. A syntactically valid SSH or firewall config can still lock out the intended user.

---

# Reboot Persistence Checks

Every destructive apply lane that changes services, SSH, firewall, Docker, Fail2Ban, hostname, or sudoers must include a reboot phase.

Use the independent SSH path for reconnect:

```bash
INSTANCE=toride-u2404
SSH_CONFIG="$HOME/.lima/$INSTANCE/ssh.config"
SSH_ALIAS="lima-$INSTANCE"

limactl shell "$INSTANCE" sudo reboot || true

for _ in $(seq 1 60); do
  if ssh -F "$SSH_CONFIG" "$SSH_ALIAS" true; then
    break
  fi
  sleep 2
done

ssh -F "$SSH_CONFIG" "$SSH_ALIAS" 'systemctl is-system-running || true'
ssh -F "$SSH_CONFIG" "$SSH_ALIAS" 'systemctl is-enabled docker 2>/dev/null || true'
ssh -F "$SSH_CONFIG" "$SSH_ALIAS" 'systemctl is-active fail2ban 2>/dev/null || true'
```

The reboot pass condition is not just that Lima restarts the VM. The pass condition is that the future supported login method still works after boot and that enabled services remain enabled.

---

# IPv6 Firewall Checks

Firewall validation must include IPv6 whenever the guest has IPv6 configured. IPv4-only success is not enough for VPS hardening.

Collect:

```bash
limactl shell <instance> ip -6 addr
limactl shell <instance> ss -tulpn
limactl shell <instance> sudo nft list ruleset
limactl shell <instance> sudo ufw status verbose
limactl shell <instance> sudo ufw status numbered
limactl shell <instance> -- bash -lc 'test -f /etc/default/ufw && grep "^IPV6=" /etc/default/ufw || true'
```

Expected checks:

* If UFW is used on Debian/Ubuntu, `/etc/default/ufw` should have `IPV6=yes` unless Toride explicitly documents an IPv4-only profile.
* `nft list ruleset` should show equivalent intent for IPv4 and IPv6, or a clearly documented reason for asymmetry.
* Attacker VM probes should test IPv4 and IPv6 separately when both addresses are present.
* Open listeners from `ss -tulpn` must be compared against firewall allow rules; a service bound to `::` may expose IPv6 even when IPv4 looks correct.

---

# SSH And Firewall Risk

Lima controls Linux guests through SSH. That means these commands depend on a working guest SSH path:

```bash
limactl shell <instance> ...
limactl copy <source> <instance>:<target>
```

Toride modules that touch SSH, UFW, nftables, Fail2Ban, users, or sudo can break the same control path the sandbox uses.

The test flow must therefore split destructive runs into phases:

1. **Smoke phase**: run modules that do not modify SSH or firewall rules.
2. **Firewall phase**: enable firewall modules only after Toride proves SSH remains allowed.
3. **SSH-hardening phase**: disable root/password login only after an independent reconnect command succeeds.
4. **Recovery phase**: if Lima SSH is broken, stop using `limactl shell` for diagnosis and reset from snapshot or delete-and-recreate.

For SSH-hardening tests, Toride must print and verify a reconnect command before applying irreversible changes. This is a hard pass/fail gate: if the future login method cannot be proven before the risky change, Toride should refuse to continue.

```bash
SSH_CONFIG="$HOME/.lima/toride-u2404/ssh.config"
ssh -F "$SSH_CONFIG" lima-toride-u2404 true
```

Lima's documented SSH host alias is `lima-<instance>`, for example `lima-default` for the `default` instance. New scripts should prefer `ssh -F ~/.lima/<instance>/ssh.config lima-<instance>` for reconnect checks.

If that exact alias is not available in the installed Lima version, inspect the instance metadata and generate the equivalent command:

```bash
limactl list toride-u2404
```

`limactl show-ssh` is officially deprecated in current Lima docs. Do not use it in normal automation; keep it only as a manual diagnostic fallback if the generated SSH config is unavailable.

Any test profile that includes SSH or firewall modules must be allowed to break the VM. The reset mechanism is the recovery path.

---

# Collecting Logs

Create a host artifact directory:

```bash
mkdir -p .sandbox-artifacts/toride-u2404
```

Collect guest state:

```bash
limactl shell toride-u2404 journalctl -b --no-pager > .sandbox-artifacts/toride-u2404/journal.txt
limactl shell toride-u2404 systemctl --failed --no-pager > .sandbox-artifacts/toride-u2404/systemd-failed.txt
limactl shell toride-u2404 cat /etc/os-release > .sandbox-artifacts/toride-u2404/os-release.txt
limactl shell toride-u2404 ss -tulpn > .sandbox-artifacts/toride-u2404/ports.txt
limactl shell toride-u2404 ip -6 addr > .sandbox-artifacts/toride-u2404/ipv6-addresses.txt
limactl shell toride-u2404 sudo nft list ruleset > .sandbox-artifacts/toride-u2404/nft-ruleset.txt || true
limactl shell toride-u2404 sudo ufw status verbose > .sandbox-artifacts/toride-u2404/ufw-status.txt || true
```

For Debian/Ubuntu:

```bash
limactl shell toride-u2404 sudo tail -n 300 /var/log/apt/history.log > .sandbox-artifacts/toride-u2404/apt-history.txt
limactl shell toride-u2404 sudo tail -n 500 /var/log/dpkg.log > .sandbox-artifacts/toride-u2404/dpkg.txt
```

For Rocky:

```bash
limactl shell toride-r10 sudo dnf history > .sandbox-artifacts/toride-r10/dnf-history.txt
limactl shell toride-r10 rpm -qa > .sandbox-artifacts/toride-r10/rpm-qa.txt
```

If Toride writes its own logs, copy those too:

```bash
limactl shell toride-u2404 -- bash -lc 'find ~/.local/state ~/.cache /var/log -iname "*toride*" 2>/dev/null'
```

---

# Destroying a Broken VM

If a test breaks SSH, systemd, package management, or Lima connectivity:

```bash
limactl stop toride-u2404 || true
limactl delete -f toride-u2404
```

Then recreate from template and remake the clean snapshot.

Do not spend time manually repairing a sandbox unless the broken state is the bug being investigated.

---

# Script Contracts

The future `dev/sandbox/lima/scripts/` commands should be thin wrappers around Lima.

## create.sh

Create or recreate one instance.

```bash
dev/sandbox/lima/scripts/create.sh ubuntu-24.04
```

Expected behavior:

* validates Lima is installed
* validates the Lima version has `snapshot`, `copy`, `start --list-templates`, and `--mount-none`
* validates the required template is present with `limactl start --list-templates`
* validates the template exists
* runs `limactl validate <template>`
* validates the image checksum when using local images
* creates the named instance
* starts it
* waits for boot readiness
* creates the `clean` snapshot, replacing an existing clean snapshot only after explicit `--recreate`

## reset.sh

Restore an instance to `clean`.

```bash
dev/sandbox/lima/scripts/reset.sh ubuntu-24.04
```

Expected behavior:

* stops the VM if needed
* applies `clean`
* starts the VM
* checks `/etc/os-release`
* checks `systemd`
* falls back to delete-and-recreate when snapshots are unavailable or fail

## run.sh

Run Toride in one VM.

```bash
dev/sandbox/lima/scripts/run.sh ubuntu-24.04 --profile sandbox
```

Expected behavior:

* builds or accepts a host binary path
* rejects macOS binaries before copying to the guest
* checks guest architecture with `uname -m`
* resets the VM first
* copies the binary into `/tmp/toride`
* verifies `/tmp/toride` executes before running tests
* runs dry-run
* optionally runs apply
* runs syntax validation gates before activating dangerous config
* records direct SSH reconnect checks before and after SSH/firewall modules
* runs the reboot persistence phase after successful apply when the profile changes persistent services
* optionally calls `netprobe.sh` for firewall profiles
* collects artifacts

## netprobe.sh

Run network probes from a separate attacker VM.

```bash
dev/sandbox/lima/scripts/netprobe.sh toride-u2404 --from toride-netprobe
```

Expected behavior:

* verifies the probe VM is not the target VM
* discovers or accepts the target VM IP
* records whether the target has IPv4, IPv6, or both
* checks that SSH is reachable when SSH should be allowed
* checks that a known closed test port is blocked
* checks that explicitly opened ports are reachable
* stores probe output under the target artifact directory
* exits non-zero when observed network behavior differs from the expected profile

## matrix.sh

Run a command against multiple VMs.

```bash
dev/sandbox/lima/scripts/matrix.sh ubuntu-24.04 debian-13 rocky-10
```

Expected behavior:

* runs each distro independently
* keeps artifacts separate
* continues after a distro fails unless `--fail-fast` is passed
* exits non-zero if any distro failed

## destroy.sh

Delete one instance.

```bash
dev/sandbox/lima/scripts/destroy.sh ubuntu-24.04
```

Expected behavior:

* stops the VM if needed
* deletes the VM
* does not delete user-supplied images
* does not delete artifacts unless `--artifacts` is passed

---

# Agent Safety Rules

An AI agent operating these sandboxes must follow these rules:

* Never run Toride apply directly on the macOS host.
* Never mount the repo writable into a destructive guest by default.
* Never reuse a VM for apply testing without restoring `clean` first.
* Never assume a distro from the instance name; check `/etc/os-release`.
* Never assume `systemd` works; check it.
* Never assume `apt` or `dnf` locks are free immediately after boot.
* Never assume Lima SSH will survive SSH-hardening or firewall tests.
* Never treat same-VM firewall probes as proof that another host is blocked.
* Never run a macOS-built Toride binary in the Linux guest.
* Never repair a broken sandbox manually unless debugging that exact failure.
* Never commit downloaded VM images or generated VM disks.
* Always prove the future SSH login path before applying SSH-hardening.
* Always collect logs before destroying a failed VM when possible.

---

# Validation Checklist

Each sandbox image is acceptable when these pass:

```bash
limactl shell <instance> cat /etc/os-release
limactl shell <instance> uname -m
limactl shell <instance> systemctl is-system-running || true
limactl shell <instance> command -v sudo
limactl shell <instance> command -v curl
limactl shell <instance> -- bash -lc 'command -v sshd || command -v ssh'
```

For Debian/Ubuntu:

```bash
limactl shell <instance> command -v apt-get
limactl shell <instance> sudo apt-get update
limactl shell <instance> -- bash -lc 'test ! -e /var/lib/dpkg/lock-frontend'
```

For Rocky:

```bash
limactl shell <instance> command -v dnf
limactl shell <instance> sudo dnf makecache
```

Snapshot validation:

```bash
limactl stop <instance>
limactl snapshot create <instance> --tag clean
limactl snapshot apply <instance> --tag clean
limactl start <instance>
limactl shell <instance> cat /etc/os-release
```

Binary validation:

```bash
limactl shell <instance> uname -m
limactl copy ./target/<linux-target>/release/toride <instance>:/tmp/toride
limactl shell <instance> chmod +x /tmp/toride
limactl shell <instance> /tmp/toride
```

SSH reconnect validation:

```bash
ssh -F ~/.lima/<instance>/ssh.config lima-<instance> true
```

Firewall validation requires an explicit network mode. For allow/block assertions, prefer attacker VM probes over checks originating inside the target guest.

---

# Real VPS Canary Lane

Before trusting a release, run one disposable real VPS canary after the Lima matrix passes. This should be a small, paid instance that can be destroyed immediately after validation.

The VPS canary should verify:

* provider image differences from Lima cloud images
* public IPv4 exposure
* public IPv6 exposure
* provider firewall behavior
* Cloudflare-only allowlists when those profiles exist
* SSH hardening with a real public address
* reboot persistence through the provider boot path
* recovery instructions before risky operations

Do not use the canary as a development loop. If it fails, capture artifacts, destroy the instance if needed, and reproduce the issue locally in Lima or a focused VPS repro.

---

# Distro-Specific Notes

## Ubuntu 24.04 LTS

This should be the first and most frequently used sandbox. It is a current common VPS baseline and already listed as a Toride target.

Recommended resources:

```text
2 CPU
4 GiB memory
24 GiB disk
```

## Ubuntu 26.04 LTS

Use this as the forward-looking Ubuntu target. It may require more memory for heavier flows.

Recommended resources:

```text
2 CPU
6 GiB memory
32 GiB disk
```

## Debian 12

Use this as the conservative Debian target. It remains important for existing VPS providers and older production machines.

Recommended resources:

```text
2 CPU
3 GiB memory
20 GiB disk
```

## Debian 13

Use this as the current Debian stable target.

Recommended resources:

```text
2 CPU
4 GiB memory
24 GiB disk
```

## Rocky Linux 9

Use this for RHEL-compatible behavior with mature package support.

Recommended resources:

```text
2 CPU
4 GiB memory
24 GiB disk
```

## Rocky Linux 10

Use this for current RHEL-compatible behavior. Verify architecture compatibility early, especially on Intel hosts, because newer enterprise distributions may raise CPU baselines.

Recommended resources:

```text
2 CPU
4 GiB memory
24 GiB disk
```

---

# Failure Modes To Expect

## Cloud-init still running

Fresh cloud images may still be initializing when the agent connects.

Check:

```bash
limactl shell <instance> cloud-init status --wait
```

## Package manager locks

APT and DNF can be busy immediately after boot.

Toride itself should handle this, but sandbox scripts may also wait before test setup.

## SSH service name differs

Debian/Ubuntu often use `ssh`; Rocky commonly uses `sshd`.

Use both:

```bash
systemctl status ssh || systemctl status sshd
```

## systemd degraded

Some cloud images boot with harmless degraded units. Capture the failed unit list before deciding:

```bash
systemctl --failed --no-pager
```

## Snapshot chain gets slow

Long snapshot chains can hurt performance. Keep only:

* `clean`
* one temporary debug snapshot when actively investigating

Delete old debug snapshots.

---

# References

* Lima docs: https://lima-vm.io/docs/
* Lima templates: https://lima-vm.io/docs/templates/
* Lima command reference: https://lima-vm.io/docs/reference/
* Debian cloud images: https://wiki.debian.org/Cloud
* Debian 13 release information: https://www.debian.org/releases/stable/
* Ubuntu 26.04 release notes: https://documentation.ubuntu.com/release-notes/26.04/
* Ubuntu release cycle: https://ubuntu.com/about/release-cycle
* Rocky Linux images: https://wiki.rockylinux.org/rocky/image/
* Rocky Linux versions: https://wiki.rockylinux.org/rocky/version/
