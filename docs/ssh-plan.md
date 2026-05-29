My updated verdict: **there is no single Rust crate that manages “all SSH things” correctly**. Build this as a layered SSH manager/doctor around OpenSSH behavior, with Rust crates for parsing and safe file handling.

## Best Rust stack

| Area                                                         | Best choice                             | Why                                                                                                                                                                                                  |
| ------------------------------------------------------------ | --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| SSH public/private keys, certs, known_hosts, authorized_keys | `ssh-key`                               | Pure Rust, supports OpenSSH public/private keys, certificates, signatures, `authorized_keys`, and `known_hosts`; key generation exists behind `ed25519`, `p256`, `rsa` features. **FIDO/sk keys can only be decoded/encoded/verified — cannot generate or sign.** Use `ssh-keygen` for all FIDO operations. ([Docs.rs][1])      |
| OpenSSH config parsing                                       | `ssh2-config-rs`                        | Pure Rust parser (fork of `veeso/ssh2-config`, no OpenSSL) aimed at `russh`, supports `IdentityFile`, `ProxyJump`, `CertificateFile`, `AddKeysToAgent`, `UseKeychain`, algorithms, agent flags, etc., but **does not fully support `Match` patterns or tokens**. Unsupported fields are accessible via `ALLOW_UNSUPPORTED_FIELDS` parse rule. ([GitHub][2]) |
| Config parser for `ssh2`/libssh2 stack                       | `ssh2-config`                           | Parses OpenSSH-style config and can query host-specific params with first-match-wins resolution, but it is older and designed around the `ssh2` crate. ([Docs.rs][3])                                                                 |
| Real SSH execution with exact user behavior                  | `openssh`                               | Wraps the system `ssh` binary, so existing `~/.ssh/config`, agent, ProxyJump, certs, etc. behave like the real CLI. **Unix-only. Password-less auth only — cannot handle interactive password/passphrase prompts.** Has two modes: process-based (spawns ssh) and native mux (connects to ControlMaster socket directly). For cross-platform or interactive use, prefer `std::process::Command` / `tokio::process::Command` calling `ssh` directly. ([Docs.rs][4])                                                                   |
| Pure Rust SSH client/server                                  | `russh`                                 | Use when you want native Rust SSH client/server, agent, key handling, forwarding-style work. Fork of Thrussh, **Tokio-based async**. Its keys module handles opening key files (with passphrase), encrypted keys, and agents. ([Docs.rs][5])                   |
| libssh2 client                                               | `ssh2`                                  | Rust bindings to libssh2; client-only, SSH protocol v2 only. Useful, but less OpenSSH-compatible than shelling out to `ssh`. ([Docs.rs][6])                                                          |
| SSH agent protocol                                           | `ssh-agent-client-rs` / `ssh-agent-lib` | `ssh-agent-client-rs` is a pure Rust **synchronous** client; `ssh-agent-lib` is **async (Tokio-based)** for custom agents and connecting to existing ones. For async apps, prefer `ssh-agent-lib`. ([Docs.rs][7])                                                        |
| Interactive commands / password prompts                      | `portable-pty`                          | Useful if your app needs to run real `ssh`, `ssh-keygen`, or `ssh-add` interactively in a pseudo-terminal. ([Docs.rs][8])                                                                            |

## What you were missing

You should not only manage private keys. A full SSH manager needs these modules:

### 1. Key inventory

Scan:

```txt
~/.ssh/id_*
~/.ssh/*.pub
~/.ssh/*-cert.pub
custom IdentityFile paths from ~/.ssh/config
agent identities from SSH_AUTH_SOCK
```

Track:

```txt
path
type: ed25519 / rsa / ecdsa / sk-ed25519 / sk-ecdsa
fingerprint sha256
comment
has_public_pair
has_certificate
encrypted/passphrase-protected
permissions
last_modified
used_by_hosts
```

OpenSSH default identity filenames include `id_rsa`, `id_ecdsa`, `id_ecdsa_sk`, `id_ed25519`, and `id_ed25519_sk`. ([OpenBSD Manual Pages][9])

Edge cases:

- **Key file format detection**: keys exist in multiple formats (OpenSSH native, PEM/RFC4716, PKCS#8, SEC1). The `ssh-key` crate handles OpenSSH format natively; PEM/PKCS#8 keys may need `ssh-keygen -i -m` conversion first. Enterprise environments often have legacy PEM keys.
- **PKCS#11 keys**: `ssh-keygen -D /path/to/pkcs11.so` downloads public keys from hardware tokens. `PKCS11Provider` in ssh_config enables automatic use. These keys have no file on disk and won't appear in `~/.ssh/id_*` scans. Must detect `PKCS11Provider` in config and list via `ssh-keygen -D`.
- **Agent-only keys**: keys loaded into the agent with no corresponding file on disk. Inventory must distinguish file-backed keys from agent-only keys.
- **SSH v1 keys**: `~/.ssh/identity` / `~/.ssh/identity.pub` are deprecated protocol v1 files. Doctor should warn if they exist, not silently ignore or misparse them.

### 2. Create new keys

Support presets:

```txt
ed25519              default modern key
rsa 4096             compatibility
ed25519-sk           hardware/FIDO key
ecdsa-sk             hardware/FIDO compatibility
```

Use either:

```txt
ssh-key crate        for pure Rust generation
ssh-keygen           for exact OpenSSH behavior, FIDO, PKCS#11, certificates
```

Important missing flags/features:

```txt
comment
passphrase
KDF rounds / -a equivalent
output path collision protection
generate .pub from private key
optional add to ssh-agent
optional add Host block to config
optional install to remote authorized_keys
```

`ssh-keygen -y` prints the public key from a private key, and `ssh-keygen` supports KDF rounds, fingerprints, known_hosts search/removal/hash, import/export, FIDO resident keys, KRLs, and certificates. ([OpenBSD Manual Pages][10])

Edge cases:

- **Passphrase change**: `ssh-keygen -p -f ~/.ssh/id_ed25519` changes the passphrase on an existing key. Supports `-N new_pass` and `-P old_pass` for non-interactive use. This is a common security operation not covered by creation or deletion.
- **Comment change**: `ssh-keygen -c -f ~/.ssh/id_ed25519` changes the comment on a key pair (both private and public). Less critical but users expect it.
- **Key format conversion**: `ssh-keygen -i -m PEM` imports from PEM format, `ssh-keygen -e -m PEM` exports to PEM format. Needed when users have legacy keys from PuTTY (PPK), OpenSSL, or older OpenSSH versions.
- **RSA key size**: the plan lists `rsa 4096` but `rsa 3072` is also common and `rsa 2048` still exists. Doctor should flag `rsa 2048` as weak.

### 3. Remove keys safely

Deleting a key is not just `rm`.

Your delete flow should offer:

```txt
remove private key
remove .pub
remove -cert.pub
remove from ssh-agent
remove IdentityFile references from ~/.ssh/config
optionally remove matching public key from remote authorized_keys
backup before delete
```

Agent removal maps to `ssh-add -d`, and deleting all loaded identities maps to `ssh-add -D`. ([OpenBSD Manual Pages][11])

### 4. Public key generation / repair

Add command:

```txt
ssh-manager key repair-public ~/.ssh/id_ed25519
```

Behavior:

```txt
read private key
derive public key
write ~/.ssh/id_ed25519.pub
preserve or regenerate comment
set permissions
```

This is a must-have because many people lose `.pub` files but still have the private key.

### 5. Multiple key management

You need a proper `Host` profile model:

```sshconfig
Host github-personal
  HostName github.com
  User git
  IdentityFile ~/.ssh/id_ed25519_personal
  IdentitiesOnly yes

Host github-work
  HostName github.com
  User git
  IdentityFile ~/.ssh/id_ed25519_work
  IdentitiesOnly yes
```

Important: OpenSSH allows multiple `IdentityFile` entries and tries them in sequence; unlike many other config directives, multiple `IdentityFile` values add to the list. `IdentitiesOnly` is needed when you want to stop the agent from offering extra keys. ([OpenBSD Manual Pages][9])

Edge cases:

- **IdentityFile accumulation semantics**: `IdentityFile` is the major exception to first-match-wins. Multiple values accumulate across Host blocks, including from `Host *` defaults. Config resolution must model this additive behavior separately from all other directives.
- **IdentitiesOnly + agent interaction**: when `IdentitiesOnly yes`, only configured `IdentityFile`/`CertificateFile` are offered, even if the agent holds more keys. When `no` (default), agent identities are offered first, then `IdentityFile` paths. The doctor must understand this to diagnose "wrong key offered" problems.
- **MaxAuthTries exhaustion**: `sshd_config` defaults `MaxAuthTries` to 6. Each key offered counts as one attempt. If the agent has many keys, they can exhaust `MaxAuthTries` before the correct key is tried. Doctor should warn when agent key count approaches typical `MaxAuthTries` values.

### 6. SSH config editor

This is a bigger problem than it looks.

You need to support:

```txt
Host blocks
Host *
Include
Match
IdentityFile
IdentityAgent
CertificateFile
UserKnownHostsFile
GlobalKnownHostsFile
ProxyJump
ProxyCommand
ForwardAgent
AddKeysToAgent
UseKeychain on macOS
LocalForward / RemoteForward / DynamicForward
ControlMaster / ControlPath / ControlPersist
CanonicalizeHostname
```

OpenSSH config resolution is order-sensitive: command line first, then user config, then system config; first obtained value wins, and more specific host blocks should usually appear before defaults. ([OpenBSD Manual Pages][9])

Big warning: `ssh2-config-rs` is useful, but it admits missing `Match` pattern and token support, while OpenSSH supports `Include`, tokens, and environment expansion in several directives. ([GitHub][2])

Edge cases:

- **Token expansion**: OpenSSH expands `%%`, `%C`, `%d`, `%H`, `%h`, `%I`, `%i`, `%j`, `%K`, `%k`, `%L`, `%l`, `%n`, `%p`, `%r`, `%T`, `%t`, `%u` in `CertificateFile`, `ControlPath`, `IdentityAgent`, `IdentityFile`, `Include`, `KnownHostsCommand`, `LocalForward`, `RemoteCommand`, `RemoteForward`, `RevokedHostKeys`, `UserKnownHostsFile`. Config values like `IdentityFile ~/.ssh/keys/%h/%u` will not resolve correctly without token substitution. This is critical for key inventory scanning.
- **Environment variable expansion**: OpenSSH expands `${ENV_VAR}` in the same directives as tokens, plus socket paths in `LocalForward`/`RemoteForward`. Config lines like `IdentityFile ${WORK_KEY_PATH}/id_rsa` must be resolved at runtime.
- **Recursive Include chains**: `Include` accepts glob(7) patterns, tilde, tokens, and env vars. Includes can nest (included files can contain their own `Include`). The config editor must follow Include chains, resolve paths relative to the including file's directory, and detect cycles.
- **`Match exec`**: the `exec` keyword in `Match` blocks runs arbitrary commands under the user's shell. Zero exit = true. This makes config resolution dynamic (e.g., network-location detection). The tool cannot fully resolve config without evaluating or simulating these.
- **`CanonicalizeHostname` double-parsing**: when enabled, OpenSSH re-parses config with the canonical hostname and re-evaluates Host/Match blocks. The tool must model this two-pass resolution to determine which blocks apply.
- **`=` separator syntax**: both `Host foo` and `Host=foo` are valid. A text-preserving editor must handle and preserve both.
- **Whitespace preservation**: configs use mixed tabs, spaces, and indentation depths. The editor must preserve the original style on every write to avoid noisy diffs.
- **ProxyCommand vs ProxyJump conflict**: OpenSSH documents that "whichever is specified first will prevent later instances of the other from taking effect." Having both is a common mistake. Doctor must detect this conflict.

So for editing, I would **not** rely only on a config parser. Use:

```txt
parser for reading/querying
custom text-preserving editor for writes
backup before mutation
append managed blocks with markers
```

Example managed block:

```sshconfig
# >>> ssh-manager github-work
Host github-work
  HostName github.com
  User git
  IdentityFile ~/.ssh/id_ed25519_work
  IdentitiesOnly yes
# <<< ssh-manager github-work
```

### 7. Known hosts manager

Must support:

```txt
list known hosts
find host
show fingerprint
remove host
hash known_hosts
scan host key
compare changed host key
support host:port format [host]:2222
support hashed entries
support GlobalKnownHostsFile
support UserKnownHostsFile
```

Use:

```txt
ssh-key crate          parse known_hosts
ssh-keygen -F host     find host
ssh-keygen -R host     remove host
ssh-keygen -H          hash known_hosts
ssh-keyscan -H host    collect host key
```

`ssh-keyscan` is designed to gather public host keys and build/verify known_hosts files, but scanning alone is **not trust verification**; it collects what the network gives you. ([OpenBSD Manual Pages][12])

Also support `UpdateHostKeys`, because OpenSSH can learn alternate host keys after authentication and update `UserKnownHostsFile`, which matters for host key rotation. ([OpenBSD Manual Pages][9])

Edge cases:

- **`@cert-authority` and `@revoked` markers**: known_hosts supports `@cert-authority` entries that trust any host certificate signed by a given CA, and `@revoked` entries that explicitly mark a host key as revoked. These have different semantics than regular entries and must be parsed/handled separately.
- **`VerifyHostKeyDNS` / SSHFP records**: OpenSSH can verify host keys against DNS SSHFP records (`ssh-keygen -r hostname` generates them). The known_hosts manager should be aware of this verification path, even if it doesn't manage DNS directly.

### 8. Authorized keys manager

This is separate from `known_hosts`.

Manage:

```txt
~/.ssh/authorized_keys
remote ~/.ssh/authorized_keys
key options
duplicate keys
comments
revoked keys
cert-authority entries
```

OpenSSH `authorized_keys` lines are basically:

```txt
options keytype base64-key comment
```

The options field can restrict keys, including `cert-authority`, forwarding controls, command restrictions, etc. ([OpenBSD Manual Pages][13])

You should support adding/removing public keys by fingerprint, not only by exact line string.

Edge cases:

- **Key options**: the options field supports `command="..."`, `from="pattern-list"`, `no-pty`, `no-port-forwarding`, `no-X11-forwarding`, `no-agent-forwarding`, `permit-open="host:port"`, `environment="NAME=value"`, `tunnel="n"`. When editing authorized_keys, the tool must parse and preserve these options rather than treating the line as opaque.
- **`RevokedKeys` in sshd_config**: server-side revocation file (text or KRL format) that refuses listed public keys. The remote doctor must check this when diagnosing "valid key rejected" scenarios.

### 9. SSH doctor: local checks

This should be a first-class command:

```bash
ssh-manager doctor
```

Checks:

```txt
~/.ssh exists
~/.ssh owner is current user
~/.ssh not group/world writable
private keys not accessible by others
public keys exist for private keys
config not writable by others
known_hosts readable/writable
authorized_keys permissions sane
IdentityFile paths exist
IdentityFile paths are not accidentally .pub files
duplicate Host aliases
Host blocks with same alias
Host * placed too early
missing IdentitiesOnly for multi-key hosts
ProxyJump host has its own usable config
SSH_AUTH_SOCK exists
agent reachable
agent has expected identities
```

OpenSSH recommends `~/.ssh` be accessible only by the user, requires user config not be writable by others, and ignores private keys if they are accessible by others. ([OpenBSD Manual Pages][14])

Edge cases:

- **StrictModes full check chain**: `sshd StrictModes yes` (default) checks the entire ownership/permission chain: home directory must not be group/world writable, `~/.ssh` must be 700, `~/.ssh/authorized_keys` must be 600/644 and owned by the user. The doctor must check all of these, not just `~/.ssh` and key files.
- **SELinux/AppArmor contexts**: on RHEL/CentOS/Fedora, incorrect SELinux security contexts on `~/.ssh` files cause authentication failures even when Unix permissions are correct. Doctor should run `ls -laZ ~/.ssh/` or `restorecon -Rvn ~/.ssh/` to check.
- **NFS home directories**: when home is on NFS with root-squashing, `sshd` (running as root) may not be able to read `~/.ssh/authorized_keys`. The doctor should detect NFS mounts and warn about this.
- **MaxAuthTries exhaustion from agent**: if the agent holds more keys than `MaxAuthTries` (default 6), the correct key may never be tried. Doctor should count agent keys and warn if the count approaches or exceeds common `MaxAuthTries` values.

### 10. SSH doctor: remote checks

Add:

```bash
ssh-manager doctor remote user@host
```

Checks:

```txt
can resolve host
can connect to port
host key status
which key was offered
which key succeeded
remote user exists
remote home exists
remote ~/.ssh permissions
remote authorized_keys exists
remote authorized_keys contains expected key
remote sshd allows PubkeyAuthentication
remote AuthorizedKeysFile setting
remote StrictModes behavior
remote logs hint
```

`sshd_config` defaults `PubkeyAuthentication` to yes, supports `AuthorizedKeysFile`, and `StrictModes` checks ownership/modes of user files and home directory before accepting login. ([OpenBSD Manual Pages][15])

Edge cases:

- **`AuthorizedKeysCommand`**: many enterprise setups use `AuthorizedKeysCommand` + `AuthorizedKeysCommandUser` to fetch keys from LDAP, Vault, or HTTP services. This is tried after `AuthorizedKeysFile`. Remote doctor must check this; otherwise it will report "key not in authorized_keys" without understanding why auth still works.
- **`AuthorizedPrincipalsCommand`**: certificate-based environments often use `AuthorizedPrincipalsCommand` to dynamically generate the principal list (from LDAP, etc.) instead of a static `AuthorizedPrincipalsFile`. Remote doctor cannot diagnose cert auth failures without checking this.
- **`Keyboard-Interactive` authentication**: `KbdInteractiveAuthentication` defaults to yes. It is used for 2FA/TOTP, PAM challenge-response, and BSD Auth. It is the default fallback after `publickey` and before `password`. Remote doctor must understand it as a distinct auth method.
- **GSSAPI/Kerberos authentication**: `GSSAPIAuthentication` is supported by both client and server. In enterprise Kerberos environments, it may be the primary auth method. Remote doctor should detect and report on GSSAPI configuration.
- **Non-standard SSH ports**: the doctor must support `ssh -p port` and config-resolved ports, not assume port 22.
- **Hosts behind jump hosts**: remote checks for hosts that require `ProxyJump` must go through the jump host, not attempt direct connection.

### 11. Agent manager

Support:

```txt
show agent status
list keys
list public keys
add key
add key with lifetime
add key with confirmation required
remove key
remove all
test key usability
detect stale SSH_AUTH_SOCK
```

OpenSSH agent can hold multiple identities, `ssh` uses them automatically, and private keys/passphrases do not go over the network; operations are performed by the agent. ([man7.org][16])

Also support newer destination-constrained keys:

```bash
ssh-add -h host
ssh-add -h jump>target
```

OpenSSH supports destination constraints since 8.9, but both the remote client/server path must cooperate when forwarding. ([OpenBSD Manual Pages][11])

Edge cases:

- **`SSH_ASKPASS`**: when SSH needs a passphrase and has no terminal, it invokes the program at `$SSH_ASKPASS`. `SSH_ASKPASS_REQUIRE` (never/prefer/force) controls this. For a TUI app, this is the correct mechanism for passphrase prompts without needing a PTY. The architecture should use `SSH_ASKPASS` pointed at the app's own passphrase handler instead of relying on `portable-pty`.
- **macOS Keychain integration**: `ssh-add --apple-use-keychain` and `--apple-load-keychain` store/retrieve passphrases in the macOS Keychain. `UseKeychain yes` in config enables automatic behavior. Agent manager must detect and respect this.
- **ControlMaster session management**: active multiplexed sessions (via `ControlPath` sockets) should be listable, inspectable (`ssh -O check`), and cleanable (`ssh -O exit`). Stale control sockets (from crashed SSH processes) are a common problem.

### 12. Install key to remote

Implement both:

```txt
safe mode: use ssh-copy-id if available
manual mode: ssh remote "mkdir -p ~/.ssh && append key && chmod ..."
```

`ssh-copy-id` appends the local public key to remote `authorized_keys` and sets appropriate permissions. ([Oracle Docs][17])

### 13. Certificates / CA support

Do not skip this if you want “all SSH related”.

Support:

```txt
user certificates
host certificates
CertificateFile
TrustedUserCAKeys
AuthorizedPrincipalsFile
cert-authority in authorized_keys
KRL revocation files
certificate expiry display
certificate principals display
```

`ssh-key` supports OpenSSH certificates and CA support, and `sshd_config` supports `TrustedUserCAKeys` for user certificates. ([Docs.rs][1])

### 14. Security key / FIDO support

Support detection, but use OpenSSH CLI for real operations:

```txt
ed25519-sk
ecdsa-sk
resident keys
touch-required
verify-required
PIN-backed keys
```

`ssh-keygen -K` handles resident FIDO keys, while `sshd_config` has FIDO-specific `PubkeyAuthOptions` like `touch-required` and `verify-required`. ([OpenBSD Manual Pages][10])

### 15. Windows/macOS/Linux differences

Do not make the doctor Unix-only.

Linux/macOS:

```txt
chmod/chown checks
ssh-agent socket
~/.ssh paths
```

macOS:

```txt
UseKeychain
AddKeysToAgent
system keychain behavior
```

Windows:

```txt
ACL checks instead of chmod
OpenSSH agent service
C:\Users\<user>\.ssh
Git Bash / WSL path confusion
```

`ssh2-config-rs` even exposes `UseKeychain` as a macOS-specific attribute. ([Docs.rs][2])

### 16. Cross-cutting: authentication methods

Do not assume pubkey is the only auth method.

The full OpenSSH auth method stack is:

```txt
1. publickey           (keys, certs, FIDO, PKCS#11)  ← plan covers this well
2. gssapi-with-mic     (Kerberos)                     ← not mentioned
3. hostbased           (host keys + .rhosts/.shosts)  ← not mentioned
4. keyboard-interactive (PAM, 2FA/TOTP, challenge)    ← not mentioned
5. password            (cleartext over encrypted channel)
```

The doctor must understand all methods to diagnose authentication failures. The `PreferredAuthentications` config directive controls which methods are tried and in what order.

### 17. Cross-cutting: port forwarding management

Port forwarding is a day-to-day SSH task.

Support:

```txt
list active forwards on a ControlMaster session (ssh -O forward -S path)
cancel a forward (ssh -O cancel -S path)
test forward connectivity (connect to local port, check it reaches remote)
parse LocalForward / RemoteForward / DynamicForward from config
detect conflicting local ports (two forwards on same local port)
```

OpenSSH escape sequence `~#` lists forwarded connections in an active session. `ssh -O forward` and `ssh -O cancel` manage forwards on multiplexed connections. ([OpenBSD Manual Pages][14])

## Final coverage checklist

Your SSH manager should have commands like this:

```bash
ssh-manager key list
ssh-manager key list --verbose           # includes format detection, PKCS#11, agent-only keys
ssh-manager key new --type ed25519 --name github-work --comment "hamza@github-work"
ssh-manager key pub ~/.ssh/id_ed25519
ssh-manager key rename old new
ssh-manager key delete github-work --remove-agent --remove-config
ssh-manager key chmod-fix
ssh-manager key change-passphrase ~/.ssh/id_ed25519
ssh-manager key change-comment ~/.ssh/id_ed25519 --comment "new-comment"
ssh-manager key convert ~/.ssh/id_rsa_pem --from PEM --to OpenSSH

ssh-manager config list
ssh-manager config get github-work
ssh-manager config add-host github-work --host github.com --user git --key ~/.ssh/id_ed25519_work
ssh-manager config remove-host github-work
ssh-manager config doctor                 # checks ProxyCommand/ProxyJump conflicts, Include chains,
                                           # token usage, IdentityFile accumulation, whitespace issues
ssh-manager config resolve github-work    # shows fully resolved config including Includes and tokens

ssh-manager known-hosts list
ssh-manager known-hosts find github.com
ssh-manager known-hosts scan github.com
ssh-manager known-hosts remove github.com
ssh-manager known-hosts hash

ssh-manager authorized-keys list --remote user@host
ssh-manager authorized-keys add --remote user@host ~/.ssh/id_ed25519.pub
ssh-manager authorized-keys remove --remote user@host --fingerprint SHA256:...

ssh-manager agent status
ssh-manager agent list
ssh-manager agent add ~/.ssh/id_ed25519 --lifetime 8h --confirm
ssh-manager agent remove ~/.ssh/id_ed25519
ssh-manager agent clear
ssh-manager agent sessions                 # list active ControlMaster sessions
ssh-manager agent session-cleanup          # clean stale control sockets

ssh-manager doctor
ssh-manager doctor host github-work
ssh-manager doctor remote user@host
ssh-manager test github-work

ssh-manager forward list                   # list active port forwards
ssh-manager forward cancel --local 8080
```

## The important architectural decision

Use **Rust crates for structure**, but use **OpenSSH CLI as the source of truth** for edge cases.

Best architecture:

```txt
ssh-key
  keys, public/private parsing, fingerprints, authorized_keys, known_hosts

ssh2-config-rs
  read/query OpenSSH config, but not trusted as a complete writer

openssh crate / std::process
  run real ssh, ssh-keygen, ssh-add, ssh-keyscan, ssh -G, ssh -vvv

ssh-agent-client-rs or ssh-agent-lib
  agent listing/removal/addition when you do not want to shell out

portable-pty
  interactive flows when passphrases/prompts are needed
```

The biggest missing piece in the Rust ecosystem is a **complete OpenSSH-compatible, comment-preserving, Include/Match/token-aware config editor**. I would build that yourself as a small AST/text-patcher instead of trusting a parser to rewrite the whole config.

## Runtime model: go async

Since toride already uses an async event loop with ratatui, the SSH subsystem should be async throughout to avoid bridging sync/async boundaries:

```txt
ssh-agent-lib          async agent client (not ssh-agent-client-rs which is sync)
russh                  async SSH client/server (Tokio-based)
tokio::process::Command  for calling ssh, ssh-keygen, ssh-add, ssh-keyscan
ssh2-config-rs         sync but fast enough to call via spawn_blocking
ssh-key                sync but fast enough to call via spawn_blocking
portable-pty           sync; wrap in spawn_blocking for interactive flows
```

## Edge case summary by severity

### Critical (will break things without handling)

| Edge case | Where | Why it breaks |
|---|---|---|
| Token expansion (`%h`, `%d`, `%u`, etc.) | Config editor, key inventory | Config values like `IdentityFile ~/.ssh/keys/%h/%u` resolve to wrong paths |
| Environment variable expansion (`${VAR}`) | Config editor, key inventory | `IdentityFile ${WORK_KEY_PATH}/id_rsa` produces broken paths |
| Recursive `Include` chains | Config editor | Cannot build complete config picture; infinite loops without cycle detection |
| `IdentityFile` accumulation | Multi-key, config resolution | Additive semantics (not first-match-wins) — wrong keys offered if modeled incorrectly |

### Important (common real-world scenarios)

| Edge case | Where | Impact |
|---|---|---|
| Key format detection (PEM/PKCS#8/OpenSSH) | Key inventory | Legacy keys appear missing |
| PKCS#11 hardware token keys | Key inventory | Incomplete key list for token users |
| `ssh-keygen -p` passphrase change | Key management | Common security operation missing |
| `Match exec` dynamic config | Config editor | Dynamic config blocks not evaluated |
| `CanonicalizeHostname` re-parsing | Config resolution | Wrong blocks matched for canonical hosts |
| `=` separator and whitespace preservation | Config editor | Config corruption on write |
| `ProxyCommand` vs `ProxyJump` conflict | Doctor | Common misconfig not detected |
| `IdentitiesOnly` + agent interaction | Doctor, multi-key | Cannot diagnose "wrong key offered" |
| `MaxAuthTries` exhaustion | Doctor | Agent with many keys fails silently |
| `AuthorizedKeysCommand` | Remote doctor | Reports key missing when external fetcher provides it |
| `AuthorizedPrincipalsCommand` | Remote doctor, certs | Cannot diagnose cert auth failures |
| `Keyboard-Interactive` (2FA/TOTP) | Remote doctor | Cannot diagnose auth fallback behavior |
| `SSH_ASKPASS` | Agent, passphrase input | Wrong mechanism for TUI passphrase prompts |
| ControlMaster session cleanup | Agent | Stale sockets not managed |
| Port forwarding management | Cross-cutting | No listing/cancelling active forwards |
| SELinux/AppArmor contexts | Doctor | False-negative permission checks on RHEL/Fedora |
| NFS home directories | Doctor | Root-squash causes false-negative auth diagnosis |

[1]: https://docs.rs/ssh-key/ "ssh_key - Rust"
[2]: https://github.com/pRizz/ssh2-config-rs "pRizz/ssh2-config-rs - GitHub"
[3]: https://docs.rs/ssh2-config "ssh2_config - Rust"
[4]: https://docs.rs/crate/openssh/latest "openssh 0.11.6 - Docs.rs"
[5]: https://docs.rs/russh/latest/russh/keys/index.html "russh::keys - Rust"
[6]: https://docs.rs/ssh2?utm_source=chatgpt.com "ssh2 - Rust"
[7]: https://docs.rs/ssh-agent-client-rs?utm_source=chatgpt.com "ssh_agent_client_rs - Rust"
[8]: https://docs.rs/portable-pty?utm_source=chatgpt.com "portable_pty - Rust"
[9]: https://man.openbsd.org/ssh_config "ssh_config(5) - OpenBSD manual pages"
[10]: https://man.openbsd.org/ssh-keygen.1 "ssh-keygen(1) - OpenBSD manual pages"
[11]: https://man.openbsd.org/ssh-add.1 "ssh-add(1) - OpenBSD manual pages"
[12]: https://man.openbsd.org/ssh-keyscan.1 "ssh-keyscan(1) - OpenBSD manual pages"
[13]: https://man.openbsd.org/sshd.8 "sshd(8) - OpenBSD manual pages"
[14]: https://man.openbsd.org/ssh.1 "ssh(1) - OpenBSD manual pages"
[15]: https://man.openbsd.org/sshd_config "sshd_config(5) - OpenBSD manual pages"
[16]: https://www.man7.org/linux/man-pages/man1/ssh-agent.1.html "ssh-agent(1) - Linux manual page"
[17]: https://docs.oracle.com/en/operating-systems/oracle-linux/openssh/openssh-CopyingPublicKeystoRemoteServers.html?utm_source=chatgpt.com "Copying Public Keys to Remote Servers"
