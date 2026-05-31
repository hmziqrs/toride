# UFW Rust Library Plan

## Product shape

Build a Rust crate that lets other Rust projects safely manage, inspect, validate, and diagnose an existing UFW firewall installation.

This is **not** a full firewall replacement.

This is **not** a full CLI product.

This is a library/package/crate that other Rust apps can embed.

The crate should expose a typed Rust API for:

* checking UFW availability
* reading UFW status
* enabling/disabling/reloading UFW
* setting default policies
* adding/removing firewall rules
* adding/removing route rules
* adding/removing application profiles
* managing logging level
* inspecting live listening ports
* checking IPv4/IPv6 coverage
* checking SSH lockout risk
* validating intended changes with dry-run
* generating doctor reports
* backing up and restoring UFW-managed config where file writes are required

## Main design rule

Use existing battle-tested solutions first.

Do not hand-roll shell execution, firewall syntax parsing, IP/CIDR parsing, service control, atomic writes, file locking, config mutation, or nftables parsing unless no decent crate exists.

Primary philosophy:

> UFW already solved the user-facing firewall interface. Our crate should safely orchestrate UFW, not become a new firewall daemon.

## Core stance

For normal firewall rules, call `ufw`.

Do not manually edit:

```text
/etc/ufw/user.rules
/etc/ufw/user6.rules
```

Those are UFW-managed files. UFW normalizes, stores, orders, and applies those rules itself. Editing them directly creates persistence and ordering bugs.

Use file editing only for:

* generated app profiles in `/etc/ufw/applications.d`
* safe edits to `/etc/default/ufw`
* safe edits to `/etc/ufw/sysctl.conf`
* optional advanced managed blocks in `before.rules`
* optional advanced managed blocks in `after.rules`
* IPv6 versions of advanced rules files

Normal allow/deny/reject/limit rules should go through the CLI.

## Non-goals

Do not build:

* a replacement for UFW
* a replacement for nftables
* a replacement for iptables
* a new packet filter
* a long-running firewall daemon
* a dashboard
* a full CLI app
* a Docker firewall manager in v1
* automatic package installation by default
* direct root escalation by default
* arbitrary shell execution
* unmanaged rewriting of `/etc/ufw/user.rules`
* broad NAT/router features in MVP

Optional example binaries are fine, but the product is the library API.

## Crate name ideas

Possible names:

* `ufw-kit`
* `ufw-rs`
* `ufw-control`
* `ufw-manager`
* `firewall-ufw`
* `uncomplicated-firewall-rs`

Best clean name: `ufw-kit`.

## Recommended workspace layout

```text
ufw-kit/
  crates/
    ufw-kit/
      src/
        lib.rs
        client.rs
        command.rs
        spec.rs
        rule.rs
        status.rs
        app_profile.rs
        config.rs
        framework.rs
        service.rs
        doctor.rs
        firewall.rs
        net.rs
        paths.rs
        diff.rs
        backup.rs
        error.rs
        report.rs
    ufw-kit-test-support/
      src/
        fake_runner.rs
        fixtures.rs
  examples/
    embed_myapp.rs
    doctor_report.rs
    ensure_web_firewall.rs
  tests/
    fixtures/
      status/
      dry_run/
      app_profiles/
      framework/
```

Keep one main public crate. Split test support only if it becomes useful.

## Public API target

The embedded API should feel like this conceptually:

```rust
let ufw = Ufw::system();

ufw.ensure_rule(
    RuleSpec::allow()
        .direction(Direction::In)
        .proto(Protocol::Tcp)
        .to_port(22)
        .comment("managed:ssh")
        .build()
)?;

ufw.ensure_rule(
    RuleSpec::allow()
        .direction(Direction::In)
        .proto(Protocol::Tcp)
        .to_port(443)
        .comment("managed:https")
        .build()
)?;

ufw.set_default_policy(Direction::Incoming, Policy::Deny)?;
ufw.set_default_policy(Direction::Outgoing, Policy::Allow)?;

let report = ufw.doctor(DoctorScope::all())?;
```

The exact API can change, but the shape should remain:

* typed
* safe
* idempotent
* dry-run capable
* rollback-aware where possible
* explicit about dangerous actions

## Core modules

### `command`

Responsible for process execution.

Use:

* `duct` for command execution
* `which` to locate binaries
* optional timeout crate for command timeouts
* optional fake runner for tests

Requirements:

* never build shell strings for normal UFW commands
* pass arguments as arrays
* capture stdout/stderr
* support dry-run mode
* support timeouts
* support redacted logs
* return structured errors
* allow tests to inject fake command output
* force `LC_ALL=C`/`LANG=C` for commands whose output will be parsed

Example internal command shape:

```rust
runner.run(CommandSpec {
    program: "ufw",
    args: vec!["status", "verbose"],
    timeout: Some(Duration::from_secs(10)),
    requires_root: true,
})
```

Avoid:

* scattered `std::process::Command`
* `sh -c`
* raw string concatenation
* mixing command execution and parsing in the same function

### `client`

Typed wrapper around the `ufw` command.

Operations:

* `version()`
* `status()`
* `status_verbose()`
* `status_numbered()`
* `show(report)`
* `enable()`
* `force_enable()`
* `disable()`
* `reload()`
* `reset()`
* `set_default_policy(direction, policy)`
* `set_logging(level)`
* `add_rule(rule)`
* `delete_rule(rule)`
* `delete_rule_number(number)`
* `insert_rule(number, rule)`
* `add_route_rule(rule)`
* `delete_route_rule(rule)`
* `app_list()`
* `app_info(name)`
* `app_update(name)`
* `app_default(app_policy)`

Expose both raw output and parsed summaries.

Do not over-trust parsing in v1. UFW output can vary by distro version, locale, IPv6 setting, and rule format.

For parsed output, run UFW with a stable C locale. Localized output should remain available through raw command APIs, but parsers should not depend on translated column names or messages.

### `spec`

Strongly typed models.

Types:

```rust
struct RuleSpec;
struct RouteRuleSpec;
struct AppProfileSpec;
struct DefaultPolicySpec;
struct LoggingSpec;
struct FrameworkRuleBlock;
struct UfwPaths;
struct UfwConfig;
struct UfwStatus;
struct UfwRule;
struct UfwDoctorReport;
```

Enums:

```rust
enum Action {
    Allow,
    Deny,
    Reject,
    Limit,
}

enum Direction {
    In,
    Out,
    Routed,
}

enum Policy {
    Allow,
    Deny,
    Reject,
}

enum Protocol {
    Tcp,
    Udp,
    Ah,
    Esp,
    Gre,
    Ipv6,
    Igmp,
}

enum ProtocolFilter {
    Any,
    Specific(Protocol),
}

enum LoggingLevel {
    Off,
    On,
    Low,
    Medium,
    High,
    Full,
}

enum RuleLogging {
    None,
    Log,
    LogAll,
}

enum AppDefaultPolicy {
    Skip,
    Allow,
    Deny,
}

enum Address {
    Any,
    Ip(IpAddr),
    Net(IpNet),
}

enum PortSpec {
    Any,
    Single(u16),
    Range { start: u16, end: u16 },
    List(Vec<PortSpec>),
    ServiceName(String),
}

enum RuleTarget {
    Any,
    Port(PortSpec),
    AppProfile(String),
}

enum RulePosition {
    Append,
    Prepend,
    Insert(u32),
}
```

Validation rules:

* port must be `1..=65535`
* port ranges must be ordered
* protocol required for port ranges and comma lists where UFW requires it
* `ProtocolFilter::Any` must render by omitting `proto`, never as `proto any`
* `ah`, `esp`, `gre`, `ipv6`, and `igmp` must not be combined with port clauses
* `ipv6` protocol rules require IPv6 addresses and `igmp` protocol rules require IPv4 addresses
* `limit` should be TCP-only in common presets
* `limit` IPv6 support must be treated carefully
* IPv6 addresses require IPv6 enabled in UFW config
* app profile rules must use `app <name>` instead of `port <name>`
* app profile rules must not specify a protocol because the profile owns protocol selection
* application default policy must use `skip|allow|deny`, not normal firewall `allow|deny|reject`
* interface names must reject whitespace, newline, slash, shell metacharacters
* comments must reject newline
* app names must reject newline and path traversal
* generated paths must stay inside UFW directories
* destructive operations require explicit opt-in
* enabling UFW over SSH requires lockout protection unless disabled explicitly

### `rule`

Responsible for turning typed rule specs into UFW argument vectors.

Example:

```rust
RuleSpec::allow()
    .direction(Direction::In)
    .on_interface("eth0")
    .proto(Protocol::Tcp)
    .from(Address::Any)
    .to(Address::Any)
    .to_port(443)
    .comment("managed:https")
```

Should produce args like:

```text
allow in on eth0 proto tcp from any to any port 443 comment managed:https
```

Never produce shell strings.

Support simple syntax:

```text
allow 22/tcp
deny 53
limit ssh/tcp
```

Support full syntax:

```text
allow in on eth0 proto tcp from 10.0.0.0/8 to any port 443 comment managed:web
deny out proto tcp to any port 25
reject in from 203.0.113.10
```

Support per-rule logging:

```text
allow in log proto tcp from 10.0.0.0/8 to any port 443
deny in log-all from 203.0.113.10
```

Support rule positioning:

```text
prepend allow from 10.0.0.10
insert 1 deny from 203.0.113.10
```

Support route syntax:

```text
route allow in on eth1 out on eth2
route allow in on eth0 out on eth1 to 12.34.45.67 port 80 proto tcp
```

Support delete by exact rule:

```text
delete allow 22/tcp
delete deny from 203.0.113.10
```

Support delete by number only as an advanced API, because numbered rules can shift.

### `status`

Responsible for parsing UFW status output.

Support:

* inactive
* active
* verbose defaults
* logging level
* numbered rules
* IPv4 rules
* IPv6 rules
* route/FWD rules
* rule comments
* interface-specific rules
* app profile rules
* new application profiles policy

Important warning:

UFW status only shows UFW-managed rules, not everything in `before.rules` or `after.rules`.

So status parsing must not claim it sees the whole kernel firewall.

For full inspection, use:

```text
ufw show raw
ufw show user-rules
ufw show before-rules
ufw show after-rules
ufw show listening
```

Expose these as separate report types.

Use `ufw show added` only for normalized command reconstruction and backup context. It does not prove the live firewall state or exact original command ordering.

### `app_profile`

Manage UFW application profiles.

UFW app profiles live in:

```text
/etc/ufw/applications.d/
```

Use generated files with a managed header:

```ini
# Managed by ufw-kit.
# Do not edit manually unless you also disable this manager.

[MyApp]
title=MyApp
description=Managed firewall profile for MyApp
ports=80/tcp|443/tcp
```

Support app profile fields:

* name
* title
* description
* ports

Port format support:

```text
22/tcp
53/udp
80/tcp|443/tcp
8000:9000/tcp
3000,3001,3002/tcp
```

Operations:

* `ensure_app_profile(profile)`
* `remove_app_profile(name)`
* `app_list()`
* `app_info(name)`
* `app_update(name)`
* `app_update_all()`
* `app_default(app_policy)`

Default behavior:

* model app default policy as `skip`, `allow`, or `deny`
* never set app default policy to `allow`
* never use `app update --add-new` unless caller explicitly opts in
* prefer generating profile, then caller explicitly adds an allow/deny rule using that app profile

### `config`

Manage UFW high-level config files.

Files:

```text
/etc/default/ufw
/etc/ufw/ufw.conf
/etc/ufw/sysctl.conf
```

Responsibilities:

* read values
* safely update owned keys
* backup before write
* atomic write
* file lock
* preserve comments where possible
* avoid rewriting unknown sections unnecessarily

Important config keys:

```text
IPV6=yes|no
DEFAULT_INPUT_POLICY=DROP|ACCEPT|REJECT
DEFAULT_OUTPUT_POLICY=ACCEPT|DROP|REJECT
DEFAULT_FORWARD_POLICY=DROP|ACCEPT|REJECT
MANAGE_BUILTINS=yes|no
IPT_SYSCTL=/etc/ufw/sysctl.conf
IPT_MODULES=...
ENABLED=yes|no
```

Do not treat config editing as a replacement for the `ufw default` command. Prefer the CLI for policy changes. Use config editing only when the CLI does not expose the setting or when preparing UFW before activation.

Changing default policies can make existing rules semantically unsafe or insufficient. Policy-changing APIs must return a migration warning and run SSH lockout checks before changing incoming policy to `deny` or `reject`.

### `framework`

Advanced UFW framework file manager.

This is optional and should not be part of the MVP unless needed.

Framework files:

```text
/etc/ufw/before.rules
/etc/ufw/after.rules
/etc/ufw/before6.rules
/etc/ufw/after6.rules
/etc/ufw/before.init
/etc/ufw/after.init
```

Use this only for things UFW CLI cannot express cleanly:

* NAT masquerading
* advanced forwarding
* raw/mangle table rules
* custom chains
* pre-UFW rules
* post-UFW rules

Never blindly rewrite whole files.

Use managed blocks:

```text
# >>> ufw-kit myapp-nat
*nat
:POSTROUTING ACCEPT [0:0]
-A POSTROUTING -s 10.0.0.0/8 -o eth0 -j MASQUERADE
COMMIT
# <<< ufw-kit myapp-nat
```

But be careful: iptables-restore files have table sections and `COMMIT` boundaries. Inserting blocks incorrectly can break firewall reloads. For v1, avoid automatic NAT unless tested heavily.

Rules:

* only edit blocks with our marker
* never delete unmanaged lines
* backup before write
* validate with `ufw --dry-run reload` where possible
* reload after write
* rollback if reload fails
* separate IPv4 and IPv6 blocks
* do not mix `*filter`, `*nat`, `*mangle`, `*raw` incorrectly

### `service`

Service manager layer.

v1 can use:

```text
systemctl is-active ufw
systemctl is-enabled ufw
systemctl start ufw
systemctl stop ufw
systemctl restart ufw
```

But the primary UFW control should still use:

```text
ufw enable
ufw disable
ufw reload
```

Optional feature flags:

```toml
systemd-zbus = ["zbus_systemd"]
service-manager = ["service-manager"]
```

Operations:

* `is_active()`
* `is_enabled()`
* `start()`
* `stop()`
* `restart()`
* `reload()`
* `journal_tail()`

Keep service-control optional because not every app should control system services.

### `firewall`

Mostly diagnostic in v1.

Do not manually modify nftables/iptables behind UFW by default.

Inspect only:

* `nft list ruleset`
* `iptables-save`
* `ip6tables-save`
* `ufw show raw`
* `ufw show listening`
* active UFW chains
* active default policies

Optional crates:

* `nftables`
* `nftables-json`
* `nf_tables`
* `iptables` if still needed
* `netlink-packet-route` only if deeper netlink support becomes necessary

Reason:

UFW can run on iptables-nft, iptables-legacy, or nft-backed systems depending on distro. Writing lower-level firewall rules yourself risks fighting UFW.

### `net`

Network types and validators.

Use:

* `ipnet` for IPv4/IPv6 CIDR types
* Rust standard library `IpAddr`
* optional `ipnet_trie` for overlap analysis
* optional interface discovery crate for NIC checks

Functions:

* parse IP
* parse CIDR
* detect private networks
* detect loopback
* detect link-local
* detect multicast
* detect unspecified
* detect public IP
* detect IPv4 vs IPv6 rule mismatch
* check if rule source overlaps trusted ranges
* check if broad allow exposes dangerous port

Rule safety helpers:

```rust
rule.exposes_port(22)
rule.exposes_port(5432)
rule.allows_from_anywhere()
rule.is_ipv6_only()
rule.is_ipv4_only()
rule.has_interface_scope()
rule.has_comment_prefix("managed:")
```

## Existing crate choices

### Process execution

Use:

* `duct`

Optional:

* `which`
* `wait-timeout`
* `process_control`

Avoid:

* raw repeated `std::process::Command`
* shell string concatenation
* `sh -c` unless explicitly gated

### Paths and filesystem

Use:

* `camino` for UTF-8 paths
* `atomic-write-file` for atomic writes
* `fs2` for file locks
* `tempfile`
* `walkdir`
* `similar` for diffs

### Config parsing/rendering

Use:

* `serde`
* `toml` only for our own config
* `indoc` for templates
* `insta` for snapshot tests

For `/etc/default/ufw`, use a conservative key-value editor that preserves comments.

For app profiles, render from typed structs.

For framework files, use managed block replacement only.

### IP and network

Use:

* `ipnet`

Optional:

* `ipnet_trie`
* `iprange`

### Firewall diagnostics

Use:

* `nftables-json` for parsing `nft --json`
* `nftables` for typed nft JSON integration
* `nf_tables` only for advanced direct netlink inspection
* `iptables-save` parsing only if required

Default should remain UFW-first.

### Errors and reports

Use:

* `thiserror`
* `miette` optionally
* `serde`
* `tracing`

### Testing

Use:

* `insta`
* `assert_fs`
* `tempfile`
* `proptest`
* fake command runner
* Docker-based integration tests

## Feature flags

Recommended features:

```toml
default = ["client", "doctor", "app-profile"]

client = []
doctor = []
app-profile = []
framework = []
service = []
systemd-zbus = []
firewall-nft = []
firewall-iptables = []
serde = []
tokio = []
```

Keep heavy and risky features out of default.

## MVP scope

MVP should include:

* binary discovery
* `ufw --version`
* `ufw status`
* `ufw status verbose`
* `ufw status numbered`
* `ufw show listening`
* `ufw show added`
* `ufw --dry-run` wrapper
* add rule
* delete exact rule
* insert rule
* prepend rule
* set default policy
* set logging level
* per-rule logging
* reload
* enable with SSH safety check
* disable with explicit dangerous opt-in
* typed `RuleSpec`
* typed `RouteRuleSpec`
* app profile generation
* app profile update
* doctor report
* fake runner tests
* snapshot tests

Do not include NAT editing in MVP unless absolutely needed.

## Nice-to-have v1.1

* NAT managed blocks
* forwarding helpers
* sysctl forwarding helpers
* Docker-aware doctor
* Traefik/Dokploy presets
* Cloudflare IP allowlist preset
* Tailscale interface preset
* WireGuard interface preset
* SSH lockout simulation
* nftables JSON inspection
* iptables-save inspection
* JSON report export
* Markdown report export
* rollback API for framework changes
* firewall diff API

## Safety model

Firewall management can lock users out of their VPS. The library must be paranoid.

Rules:

* no shell by default
* no destructive reset unless explicit
* no disable unless explicit
* no force-enable unless explicit
* no deleting by rule number unless explicit
* no permanent unmanaged file mutation
* no editing `user.rules` directly
* no editing unmanaged framework lines
* no enabling UFW over SSH unless SSH allow rule exists or caller bypasses
* no default incoming deny over SSH unless SSH allow rule exists or caller bypasses
* no default outgoing deny unless caller confirms required outbound ports
* no broad allow of dangerous ports without warning
* no app default allow
* no logs at high/full without disk warning
* no secrets in comments
* no writing outside `/etc/ufw` and `/etc/default/ufw`

## SSH lockout protection

Before enabling UFW or changing default incoming policy to deny/reject, doctor must check:

* active SSH connection detected if possible
* SSHD listening port
* UFW allows that port
* UFW allows the source IP or at least allows from anywhere
* rule covers correct interface
* rule covers IPv4/IPv6 as needed
* default incoming policy is safe
* route/forwarding changes do not cut off access path

Suggested safe guard:

```rust
EnableOptions {
    require_ssh_allow_rule: true,
    ssh_ports: vec![22],
    trusted_sources: vec![],
    allow_force: false,
}
```

If not safe, return a hard error:

```text
Refusing to enable UFW: SSH port 22 is not allowed.
Add allow rule first or pass explicit override.
```

## Doctor module

The doctor should be the strongest feature.

Return structured findings, not only text.

```rust
enum Severity {
    Ok,
    Info,
    Warning,
    Error,
    Critical,
}

struct Finding {
    id: &'static str,
    severity: Severity,
    title: String,
    detail: String,
    fix: Option<String>,
}
```

### Doctor categories

#### Binary checks

* `ufw` exists
* `ufw --version` works
* `iptables` exists if UFW backend needs it
* `ip6tables` exists if IPv6 enabled
* `nft` exists if nft diagnostics enabled
* `systemctl` exists if service checks enabled

#### Service checks

* UFW service active
* UFW service enabled
* UFW reports active/inactive consistently
* UFW reload succeeds
* boot integration exists
* service logs show recent failures

#### Config checks

* `/etc/default/ufw` exists
* `/etc/ufw/ufw.conf` exists
* `/etc/ufw/sysctl.conf` exists
* `/etc/ufw/before.rules` exists
* `/etc/ufw/after.rules` exists
* IPv6 files exist when IPv6 enabled
* config files not world-writable
* generated files have safe ownership
* generated app profiles have managed headers

#### Policy checks

* default incoming policy
* default outgoing policy
* default routed policy
* incoming deny/reject recommended for servers
* outgoing allow usually safest for general VPS
* routed deny unless forwarding/router use case
* warn on outgoing deny without DNS/NTP/package manager allowances

#### Rule checks

* duplicate rules
* shadowed rules
* broad allow from anywhere
* dangerous exposed ports
* missing comments on managed rules
* managed rule comment prefix present
* IPv4 rule exists but IPv6 equivalent missing
* IPv6 rule exists but IPv4 equivalent missing
* interface-scoped rule references missing interface
* delete-by-number risks
* rule order risks
* app profile rule points to missing profile

Dangerous ports to warn about by default:

```text
22      SSH
2375    Docker API plaintext
2376    Docker API TLS
5432    Postgres
3306    MySQL
6379    Redis
27017   MongoDB
9200    Elasticsearch
9300    Elasticsearch transport
11211   Memcached
8080    common admin/dev
9000    MinIO/admin/dev
9090    Prometheus
3000    Grafana/dev
```

Do not block these automatically. Warn.

#### SSH checks

* SSHD listening port detected
* UFW allows SSH
* `limit ssh/tcp` exists if requested
* allow rule source is not too broad if trusted IPs provided
* IPv6 SSH access covered
* Tailscale/WireGuard SSH path covered if configured
* enabling UFW will not cut the current session

#### IPv6 checks

* `IPV6=yes`
* IPv6 UFW rules exist
* IPv6 default policies sane
* IPv6 listening ports are not exposed unexpectedly
* IPv6 route rules checked
* IPv6 disabled behavior understood
* dual-stack apps have dual-stack firewall coverage

This is important because VPS providers often give public IPv6 by default.

#### Logging checks

* logging enabled
* logging level safe
* warn on `high` or `full` on busy servers
* `/var/log/ufw.log` exists if rsyslog is used
* journald logs available
* log rotation exists
* disk not filling from firewall logs
* per-rule logging used intentionally

#### App profile checks

* generated app profiles parse
* app profile names valid
* app profile ports valid
* app profile update succeeds
* no app default allow unless explicitly chosen
* stale profiles from removed apps
* profile rule references missing profile

#### Framework checks

Only if framework feature enabled:

* managed blocks valid
* no duplicate markers
* before/after rule files contain required COMMIT lines
* IPv4 and IPv6 files separated correctly
* UFW reload succeeds after changes
* NAT blocks placed correctly
* forwarding sysctl enabled when route/NAT requires it
* default routed policy matches forwarding intent

#### Docker checks

Optional but important for VPS usage.

Check:

* Docker is installed
* Docker firewall backend and settings detected: `iptables`, `ip6tables`, and `firewall-backend`
* UFW rules may not protect published Docker ports as expected
* published ports are visible via `ss`/Docker inspect
* recommend binding containers to `127.0.0.1` behind reverse proxy
* warn when Docker publishes `0.0.0.0:PORT`
* warn when Docker publishes `[::]:PORT`
* warn when Docker direct-routing or routed gateway mode may expose published container ports
* warn that disabling Docker firewall management is likely to break bridge networking unless replacement rules exist
* detect Dokploy/Traefik common setup
* recommend provider firewall or Docker-specific firewall strategy if needed

This module should be warnings only in v1. Docker + UFW is a known footgun.

#### Reverse proxy checks

For NGINX, Traefik, Caddy, Dokploy:

* only 80/443 exposed publicly
* app ports bound to localhost/internal network
* dashboard ports not public
* SSH remains allowed
* provider firewall matches UFW
* Cloudflare proxied domains do not mean ports are private
* IPv6 exposure checked

#### Routing/forwarding checks

For route rules:

* `DEFAULT_FORWARD_POLICY`
* IPv4 forwarding sysctl
* IPv6 forwarding sysctl
* interface names exist
* in/out interfaces are not swapped
* NAT masquerade exists if needed
* UFW route rule exists
* kernel forwarding active now
* config persists after reboot

#### Permission checks

* `/etc/ufw` not world-writable
* `/etc/default/ufw` not world-writable
* generated app profiles not world-writable
* generated backups not world-writable
* framework files not world-writable
* owner is root or expected admin user
* comments do not leak secrets

## Idempotent rule management

UFW itself does not provide a perfect stable rule ID system. Our crate should create idempotency using comments and normalized rule specs.

Recommended managed comment format:

```text
managed-by=ufw-kit app=myapp id=https
```

Or shorter:

```text
ufw-kit:myapp:https
```

For `ensure_rule`:

1. build normalized rule
2. run `ufw status numbered`
3. parse existing rules
4. find rule by managed comment
5. if exact match exists, do nothing
6. if managed comment exists but rule differs, delete old exact rule or numbered rule with caution
7. add new rule
8. verify status

For unmanaged user rules, never modify unless caller explicitly asks.

## Apply workflow

All mutating operations should follow this flow:

1. validate typed spec
2. build UFW argv
3. run `ufw --dry-run ...`
4. inspect dry-run output for obvious failure
5. perform change
6. verify live state with `ufw status`, `ufw status numbered`, `ufw show raw`, or a targeted report
7. return structured apply report

Use `ufw show added` only as supporting evidence for normalized UFW-managed rules. It is not a live-state verification source.

If the operation changes default incoming, outgoing, or routed policy, include an explicit warning that existing rules may need migration or review under the new default.

For file-backed operations:

1. validate typed spec
2. render new file/block in memory
3. acquire lock
4. read current file
5. verify ownership/permissions
6. compute diff
7. backup
8. atomic write
9. run dry-run reload if supported
10. reload UFW
11. verify
12. rollback if reload fails

## Remove workflow

Removing rules safely is tricky.

Support three modes:

### Exact-rule delete

Preferred for normal rules.

```text
ufw delete allow 22/tcp
```

Use when the original typed spec is known.

### Managed-comment delete

Good for rules created by this crate.

Flow:

1. parse numbered status
2. find managed comment
3. delete by number from bottom to top if multiple
4. verify comment gone

Deleting from bottom to top avoids number shifting problems.

### Number delete

Advanced only.

Require explicit opt-in because rule numbers shift.

```rust
DeleteOptions {
    allow_numbered_delete: true,
}
```

## Reset workflow

`ufw reset` is destructive.

Expose it only as:

```rust
reset(ResetOptions {
    force: true,
    backup_first: true,
})
```

Default should refuse.

Before reset:

* capture `ufw status numbered`
* capture `ufw show added`
* capture `/etc/default/ufw`
* capture app profiles managed by crate
* capture managed framework blocks
* write backup bundle
* then reset

## Enable workflow

`ufw enable` can break remote access.

Safe enable flow:

1. check current status
2. detect if running over SSH if possible
3. check SSH allow rules
4. check IPv4 and IPv6 SSH coverage
5. check default incoming policy
6. run `ufw --dry-run enable`
7. require explicit force if SSH unsafe
8. run `ufw enable` or `ufw --force enable`
9. verify active

Default should never force-enable.

## Disable workflow

Disabling firewall is dangerous.

Expose:

```rust
disable(DisableOptions {
    require_explicit_confirmation: true,
})
```

Library callers can pass explicit confirmation. Do not make it easy accidentally.

## Logging workflow

Support:

```rust
ufw.set_logging(LoggingLevel::Low)
ufw.set_logging(LoggingLevel::Off)
```

Also support per-rule logging on rule specs:

```rust
RuleSpec::allow()
    .proto(Protocol::Tcp)
    .to_port(443)
    .logging(RuleLogging::Log)
    .build()
```

Doctor should warn:

* `medium`, `high`, and `full` can be noisy
* logs may fill disk
* per-rule logging may be better than global high/full
* logging may go to kernel/syslog/journald depending distro

## Application profile workflow

Creating app profile:

1. validate name
2. validate ports
3. render profile
4. write to `/etc/ufw/applications.d/<namespace>-<name>`
5. run `ufw app update <name>`
6. verify `ufw app info <name>`

Applying app profile:

```rust
ufw.ensure_rule(
    RuleSpec::allow()
        .app("MyApp")
        .from(Address::Any)
        .comment("ufw-kit:myapp")
        .build()
)
```

Do not auto-allow new app profiles by default.

## Presets

Presets should generate specs, not execute magic.

Possible presets:

### SSH preset

```rust
Preset::Ssh {
    port: 22,
    source: Address::Any,
    limit: true,
}
```

Generates:

```text
ufw limit 22/tcp comment ufw-kit:ssh
```

Or allow from trusted IPs only.

### Web server preset

```rust
Preset::WebPublic
```

Generates:

```text
allow 80/tcp
allow 443/tcp
```

### Reverse proxy preset

For Traefik/NGINX/Caddy/Dokploy:

* allow 80/tcp
* allow 443/tcp
* keep app ports private
* warn about dashboard ports
* optional allow SSH

### Tailscale admin preset

* allow on `tailscale0`
* optionally deny admin ports on public interfaces
* allow SSH only from Tailscale network

### WireGuard preset

* allow UDP WireGuard port
* optional route rules
* optional forwarding checks

### Database private preset

For Postgres/MySQL/Redis:

* deny public by default
* allow only trusted CIDR
* warn if bound to `0.0.0.0`
* check listening report

### Monitoring private preset

For Prometheus/Grafana/node exporter:

* allow only trusted CIDR or Tailscale
* warn on public exposure

## Docker and Dokploy notes

For VPS users with Dokploy/Traefik/Docker, UFW alone may not behave like expected because Docker can publish ports and manage its own firewall rules. Docker uses iptables by default, can also use an nftables backend, and published container traffic can be diverted before normal UFW input/output rules see it.

The library should include a doctor module that checks:

* Docker installed
* Docker firewall backend and daemon firewall settings
* containers publishing public ports
* containers using direct routing or routed gateway mode
* Traefik published ports
* Dokploy dashboard exposed
* app containers exposed outside reverse proxy
* IPv6 publishes
* Docker daemon port exposed
* provider firewall mismatch

Recommended app-level advice from the crate report:

```text
Bind apps to internal Docker networks.
Expose only Traefik on 80/443.
Bind admin dashboards to localhost or Tailscale.
Use provider firewall where possible.
Use UFW as host firewall, not as the only Docker isolation layer.
```

Do not attempt to fully solve Docker firewalling in v1.

## Advanced NAT/forwarding plan

This belongs after MVP.

Support typed NAT spec:

```rust
MasqueradeSpec {
    source: IpNet,
    out_interface: "eth0",
    ip_version: IpVersion::V4,
}
```

Support typed forwarding spec:

```rust
ForwardSpec {
    in_interface: "wg0",
    out_interface: "eth0",
    destination: Address::Any,
    proto: ProtocolFilter::Any,
}
```

Apply requires:

* route rule through UFW CLI
* `DEFAULT_FORWARD_POLICY` check
* sysctl forwarding enabled
* managed NAT block in `before.rules`
* UFW reload
* live verification

This must have integration tests before release.

## Reports

Expose report types:

```rust
enum UfwReport {
    Raw,
    Builtins,
    BeforeRules,
    UserRules,
    AfterRules,
    LoggingRules,
    Listening,
    Added,
}
```

Methods:

```rust
ufw.show(UfwReport::Listening)
ufw.show(UfwReport::Added)
ufw.show(UfwReport::Raw)
```

Use reports for doctor diagnostics.

Important report semantics:

* `Raw` is the best UFW-provided view of the live firewall.
* `Listening` is live socket state plus candidate matching rules.
* `Added` reconstructs normalized UFW command-line rules and does not prove running firewall status.

## Parsing strategy

Do not overfit to one distro’s output.

Run parsable UFW commands with `LC_ALL=C` and `LANG=C`. Treat localized output as raw text only unless explicit locale-specific fixtures exist.

Parsing levels:

### Level 1

Return raw stdout/stderr.

### Level 2

Parse simple status:

* active/inactive
* logging
* default policies
* rules table lines

### Level 3

Parse numbered rules and comments.

### Level 4

Parse `show added` as normalized UFW commands.

### Level 5

Parse raw iptables/nftables diagnostics.

MVP should do levels 1–3 and partial level 4.

## Testing plan

### Unit tests

* rule validation
* port validation
* protocol validation
* address validation
* interface validation
* comment validation
* app profile rendering
* command arg generation
* policy arg generation
* app default policy arg generation
* logging arg generation
* per-rule logging arg generation
* parser fixtures

### Snapshot tests

Snapshot:

* generated UFW args
* generated app profile
* doctor report
* dry-run report
* parsed status
* parsed numbered status
* parsed listening report
* generated framework block

### Fake command tests

Simulate:

* missing `ufw`
* inactive firewall
* active firewall
* failed dry-run
* failed add rule
* failed delete rule
* failed reload
* failed app update
* malformed output
* permission denied
* timeout

### Integration tests

Run in Docker/VM where possible:

* install UFW
* check version
* enable safely in isolated environment
* add allow rule
* add deny rule
* add limit rule
* add route rule
* delete exact rule
* insert rule
* create app profile
* update app profile
* reload
* parse status
* parse numbered status
* reset after test

Some tests need root or `CAP_NET_ADMIN`, so mark them ignored by default or run in privileged CI jobs.

## CI plan

Basic CI:

* fmt
* clippy
* unit tests
* snapshot tests
* fake runner tests

Optional privileged CI:

* Docker Ubuntu latest
* Docker Debian latest
* real UFW integration tests
* IPv6 enabled test
* app profile test
* service test

## Documentation deliverables

The crate should ship:

* README with “library, not CLI” positioning
* quickstart
* safety model
* SSH lockout warning
* dry-run examples
* rule examples
* route rule examples
* app profile examples
* doctor examples
* Docker/Dokploy warning
* IPv6 guide
* rollback guide
* feature flag table
* integration test guide
* root/CAP_NET_ADMIN explanation

## Suggested implementation order

### Sprint 1: Foundations

* crate skeleton
* error types
* command runner trait
* duct runner
* fake runner
* binary discovery
* basic UFW client

### Sprint 2: Typed rules

* `RuleSpec`
* `RouteRuleSpec`
* validators
* UFW argv renderer
* dry-run support
* add/delete/insert rule

### Sprint 3: Status parsing

* status parser
* verbose parser
* numbered parser
* show report wrapper
* basic idempotency by comment

### Sprint 4: Safety and doctor

* SSH lockout checks
* binary checks
* service checks
* policy checks
* rule checks
* IPv6 checks
* logging checks

### Sprint 5: App profiles

* `AppProfileSpec`
* renderer
* atomic writes
* backup
* app update
* app info verification

### Sprint 6: Integration quality

* Docker/VM tests
* docs
* examples
* CI matrix
* presets

### Sprint 7: Advanced framework

* managed blocks
* sysctl helpers
* NAT helper
* forwarding helper
* rollback after failed reload

## Final audit checklist

Before calling v1 done, the crate must support:

* locate `ufw`
* read UFW version
* read active/inactive status
* read verbose status
* read numbered status
* add allow rule
* add deny rule
* add reject rule
* add limit rule
* delete exact rule
* insert rule
* prepend rule
* add route rule
* delete route rule
* set incoming default policy
* set outgoing default policy
* set routed default policy
* set logging level
* set per-rule logging
* set app default policy with `skip|allow|deny`
* reload UFW
* safe enable workflow
* explicit disable workflow
* dry-run before mutation
* typed IP/CIDR validation
* typed port validation
* typed protocol validation
* app profile target validation
* typed interface validation
* typed comment validation
* app profile generation
* app profile update
* doctor report
* SSH lockout protection
* IPv6 checks
* rule duplicate checks
* broad exposure warnings
* Docker exposure warnings
* service checks
* permission checks
* backup before file write
* rollback after failed file-backed apply
* no shell string command execution by default
* no direct `user.rules` editing
* no unmanaged framework mutation
* no destructive reset without explicit opt-in
