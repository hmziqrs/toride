# Rust System Status Library Plan

## Goal

Build a Rust crate/package/library for collecting system status, system specs, hardware details, and live task-manager-style telemetry.

This is **not** a full CLI, not a TUI, not a dashboard, and not a monitoring SaaS.

The crate should expose clean Rust APIs that other projects can embed.

It should provide:

* CPU usage
* per-core CPU usage
* CPU specs
* CPU topology
* memory usage
* memory specs
* swap usage
* disk usage
* disk I/O
* disk health where possible
* storage device inventory
* filesystem/mount info
* network interfaces
* network I/O
* network addresses
* process list
* per-process CPU/memory/I/O
* process tree
* GPU identity
* GPU utilization where supported
* GPU memory usage where supported
* GPU temperature where supported
* OS info
* OS version
* kernel version
* hostname
* uptime
* boot time
* battery info
* temperatures
* fans/sensors where supported
* virtualization/container detection
* cgroup/container limits
* system capabilities report

## Main design rule

Use existing crates first.

Do not home-cook:

* process scanning
* `/proc` parsing
* disk counter parsing
* network interface parsing
* Windows WMI bindings
* macOS framework bindings
* NVIDIA NVML bindings
* CPU feature detection
* DMI/SMBIOS parsing
* PCI enumeration
* battery handling
* command execution

Only custom-code:

* normalized data model
* provider abstraction
* capability detection
* snapshot/delta engine
* refresh policy
* privacy redaction
* error mapping
* testing fixtures
* docs and examples

## Crate name ideas

Possible names:

* `system-status-kit`
* `sys-status`
* `machine-status`
* `host-status`
* `taskman-core`
* `sysprobe`
* `hostprobe`
* `machine-probe`

Best clean name: `sysprobe`.

## Product shape

The library should have two layers:

### Simple API

For normal users who just want status.

```rust
let probe = SysProbe::new();

let snapshot = probe.snapshot()?;

println!("{:?}", snapshot.cpu.total_usage);
println!("{:?}", snapshot.memory.used);
println!("{:?}", snapshot.disks);
```

### Advanced API

For long-running apps, dashboards, agents, and monitoring tools.

```rust
let mut collector = Collector::builder()
    .cpu(true)
    .memory(true)
    .disks(true)
    .network(true)
    .processes(true)
    .gpu(true)
    .interval(Duration::from_secs(1))
    .build()?;

let first = collector.snapshot()?;
let second = collector.snapshot_after_interval()?;
let delta = second.diff(&first)?;
```

## Core principle: snapshots and deltas

Some metrics are instant values.

Examples:

* total memory
* used memory
* disk total size
* disk free size
* OS version
* CPU brand
* process name

Some metrics are counters that require two samples.

Examples:

* CPU usage
* disk read/write rate
* network upload/download rate
* per-process CPU usage
* per-process I/O rate

The library must model this clearly.

Do not pretend one sample can produce accurate rates.

## Workspace layout

```text
sysprobe/
  crates/
    sysprobe/
      src/
        lib.rs
        snapshot.rs
        delta.rs
        collector.rs
        provider.rs
        capabilities.rs
        units.rs
        error.rs
        privacy.rs
        os.rs
        cpu.rs
        memory.rs
        storage.rs
        disk_io.rs
        network.rs
        process.rs
        gpu.rs
        battery.rs
        sensors.rs
        hardware.rs
        virtualization.rs
        cgroup.rs
        doctor.rs
        report.rs
    sysprobe-test-support/
      src/
        fixtures.rs
        fake_provider.rs
  examples/
    snapshot.rs
    task_manager_loop.rs
    process_table.rs
    gpu_status.rs
    doctor.rs
  tests/
    fixtures/
      linux_proc/
      linux_sys/
      windows_wmi/
      macos_system_profiler/
```

Keep one main public crate. Split test helpers only if needed.

## Recommended crate stack

### Default foundation

Use `sysinfo` as the first/default provider.

It should cover the basic cross-platform surface:

* system info
* CPU usage
* per-core CPU usage
* memory
* swap
* processes
* disks
* networks
* components where available
* users where available

### Deep Linux telemetry

Use `procfs` for Linux-specific deeper stats when existing cross-platform APIs are not enough.

* `/proc/stat` detailed CPU counters
* `/proc/meminfo` detailed memory stats
* `/proc/diskstats` per-device I/O counters
* `/proc/net/dev` detailed network counters
* `/proc/[pid]/` deep per-process details
* `/sys/class` device class info

Note: `systemstat` and `psutil` overlap heavily with `sysinfo`. Only add them if specific gaps are found during implementation. Prefer `sysinfo` + `procfs` as the primary stack.

### OS info

Use `os_info` for:

* OS type
* OS version
* OS edition
* bitness
* codename where available

Also expose:

* kernel version
* hostname
* boot time
* uptime
* architecture
* target triple if available
* libc/musl/glibc detection where possible

### CPU specs

Use:

* `sysinfo` for CPU brand/frequency/core counts
* `raw-cpuid` for x86/x86_64 CPU features
* `cpufeatures` for runtime feature detection where useful
* `hwlocality` for topology/NUMA/cache/socket layout (alpha but only maintained hwloc binding; gate behind feature flag, requires system `libhwloc`)

CPU data to expose:

* vendor
* brand/model
* architecture
* physical cores
* logical cores
* sockets/packages
* cores per socket
* threads per core
* base frequency
* current frequency
* max frequency where available
* per-core frequency where available
* cache info where available
* instruction features
* NUMA topology where available
* virtualization flags where available

### Memory specs

Use:

* `sysinfo` for memory usage
* `dmidecode` for memory hardware details on systems where SMBIOS/DMI is available
* `hwlocality` for NUMA memory topology

Memory data to expose:

* total memory
* used memory
* free memory
* available memory
* cached memory where available
* buffers where available
* swap total
* swap used
* swap free
* memory pressure where available
* DIMM count where available
* DIMM size where available
* memory type where available
* memory speed where available
* manufacturer/serial optional and redacted by default
* NUMA nodes where available

### Storage and filesystems

Use:

* `sysinfo` for disks and mount usage
* `procfs` for Linux disk I/O counters (`/proc/diskstats`)
* `rsblkid`/`blkid` for filesystem UUID/label/type on Linux (via `duct`)
* `udev` for Linux disk metadata (Linux-only, requires system `libudev`)
* optional `smartctl` via `duct` command execution for SMART health (gate behind `commands` feature; do NOT use `smartctl-rs` — only 124 downloads, too immature)

Storage data to expose:

* mount point
* filesystem type
* total bytes
* used bytes
* free bytes
* available bytes
* removable flag where available
* disk name
* physical device path
* disk model
* disk serial optional and redacted by default
* disk type: HDD/SSD/NVMe/USB/virtual where available
* partition info
* read bytes
* written bytes
* read operations
* write operations
* busy time
* I/O rate from delta snapshots
* SMART health if available
* temperature if available
* wear percentage if available

### Network

Use:

* `sysinfo` for basic network counters
* `netdev` for network interface metadata (cross-platform, 1.85M downloads, actively maintained; avoid `getifs` — 9 of 13 versions yanked)
* `procfs` for Linux network I/O counters (`/proc/net/dev`)
* `rtnetlink` for Linux advanced network details

Network data to expose:

* interface name
* display name
* description
* MAC address optional and redacted by default
* IPv4 addresses
* IPv6 addresses
* gateway where available
* DNS where available
* MTU
* link status
* speed where available
* duplex where available
* packets received
* packets sent
* bytes received
* bytes sent
* errors
* drops
* upload/download rate from delta snapshots
* active connections optional
* listening ports optional
* Wi-Fi SSID/signal optional per platform

### Processes

Use:

* `sysinfo` as default
* `procfs` for Linux-specific deep process details
* `procfs` for Linux-specific deep process details

Process data to expose:

* PID
* parent PID
* process name
* executable path
* command line optional and redacted by default
* current working directory optional
* user
* status
* start time
* runtime
* CPU usage
* memory RSS
* virtual memory
* disk read/write bytes
* open files where available
* thread count
* file descriptor count where available
* environment variables never collected by default
* process tree
* children
* kill/terminate support should be optional and gated

Important: process command lines can contain API keys and secrets. Redact by default.

### GPU

GPU is the hardest part. Do not pretend it is solved equally across all platforms.

Use a provider model.

Default GPU identity providers:

* `nvml-wrapper` for NVIDIA (primary, 4M downloads, well-maintained)
* `wgpu` for cross-platform adapter enumeration (feature-gated, heavy dependency)
  * Use `Instance::enumerate_adapters(backends)` — no device creation needed
  * Returns name, vendor, device type (DiscreteGpu, IntegratedGpu, etc.)
* `pci-info` + `pci-ids` for PCI-based GPU enumeration fallback
* `duct` + `lspci`/`system_profiler` as zero-dep fallback for basic identity

NVIDIA metrics provider:

* `nvml-wrapper` (GPU name, memory, temperature, utilization, processes)

Note: Do NOT use `all-smi` — it is a CLI binary, not a library. Do NOT use `gfxinfo` — only 6K downloads, too immature. There is no mature cross-platform GPU metrics library in Rust; NVIDIA via NVML is the only well-supported path.

GPU data to expose:

* vendor
* model
* device ID
* PCI bus ID where available
* driver version where available
* backend/API source
* dedicated/integrated/virtual type where available
* total VRAM
* used VRAM
* free VRAM
* GPU utilization
* memory utilization
* encoder utilization where available
* decoder utilization where available
* temperature
* fan speed
* power draw
* power limit
* clock speed
* per-process GPU memory where available

Provider support expectations:

* NVIDIA Linux/Windows: strong via NVML
* AMD Linux: partial/optional
* Intel GPU: partial/optional
* Apple Silicon: identity easier than utilization; utilization may need platform-specific provider or external tool
* generic cross-platform: identity only is realistic

### Battery

Use `starship-battery` (actively maintained fork of the abandoned `battery` crate; 1.59M downloads, maintained by the Starship prompt project).

Expose:

* battery count
* vendor/model where available
* state
* charge percentage
* energy
* energy full
* energy full design
* voltage
* cycle count where available
* time to empty
* time to full
* health estimate where available

### Sensors

Use:

* `sysinfo` components where enough
* `lm-sensors` on Linux (v0.5.1, 23K downloads, active but niche; gate behind feature flag)
* platform-specific providers where needed

Expose:

* CPU package temperature
* per-core temperature where available
* GPU temperature where available
* motherboard temperature where available
* fan RPM
* voltage sensors
* thermal throttling flags where available

Do not make sensors a hard dependency. Sensor support varies heavily.

### Hardware inventory

Use:

* `dmidecode` for SMBIOS/DMI parsing (v1.0.1, 159K downloads, reached 1.0 stable)
* `pci-info` for PCI device enumeration
* `pci-ids` for PCI vendor/device name resolution
* `udev` for Linux device metadata (Linux-only, requires system `libudev`)

Expose:

* manufacturer
* product name
* board name
* BIOS/UEFI vendor
* BIOS/UEFI version
* chassis type
* CPU package info
* memory slots
* PCI devices
* USB devices optional
* storage devices
* GPU devices
* network adapters

Redact by default:

* serial numbers
* UUIDs
* asset tags
* MAC addresses

### OS and runtime info

Expose:

* OS name
* OS type
* OS version
* OS edition
* codename
* kernel version
* hostname
* architecture
* bitness
* boot time
* uptime
* timezone
* locale optional
* current user optional
* running as root/admin flag
* container detected flag
* VM detected flag
* WSL detected flag
* systemd detected flag on Linux

### Virtualization and containers

Expose:

* running in Docker
* running in LXC
* running in containerd
* running in Kubernetes
* running in WSL
* running in VM
* hypervisor vendor where available
* cgroup version
* CPU quota
* memory limit
* swap limit
* blkio limits
* cpuset limits

Use:

* cgroup filesystem readers
* `cgroups-rs` for cgroups v1 and v2 (4.98M downloads, maintained by kata-containers)
* Linux `/proc/1/cgroup`
* DMI hints via `dmidecode`
* WSL environment hints

## Provider model

Use provider traits internally.

```rust
trait CpuProvider {
    fn cpu_static(&self) -> Result<CpuStatic>;
    fn cpu_sample(&mut self) -> Result<CpuSample>;
}

trait MemoryProvider {
    fn memory_sample(&mut self) -> Result<MemorySample>;
}

trait GpuProvider {
    fn gpu_static(&self) -> Result<Vec<GpuStatic>>;
    fn gpu_sample(&mut self) -> Result<Vec<GpuSample>>;
}
```

Public users should not have to care about providers unless they want custom providers.

## Capability detection

The library must expose what is supported on the current machine.

Example:

```rust
let caps = probe.capabilities()?;

assert!(caps.cpu.usage);
assert!(caps.memory.usage);
assert!(caps.gpu.nvidia_nvml_available == false);
```

Capability model:

```rust
struct Capabilities {
    os: OsCapabilities,
    cpu: CpuCapabilities,
    memory: MemoryCapabilities,
    storage: StorageCapabilities,
    network: NetworkCapabilities,
    process: ProcessCapabilities,
    gpu: GpuCapabilities,
    sensors: SensorCapabilities,
    battery: BatteryCapabilities,
}
```

This prevents fake “unknown zero” values.

If a metric is unsupported, return `Unsupported`, not `0`.

## Error model

Use `thiserror`.

Error categories:

* unsupported platform
* missing provider
* permission denied
* command missing
* command failed
* parse error
* stale sample
* provider timeout
* partial data
* redacted data
* OS API error

The library should support partial success.

Example: CPU and memory can succeed while GPU fails.

## Privacy model

Default safe behavior:

* redact serial numbers
* redact MAC addresses
* redact machine UUIDs
* redact asset tags
* redact full process command lines
* never collect environment variables by default
* never collect shell history
* never collect browser data
* never collect usernames unless enabled
* never expose exact public IP by default unless requested

Privacy modes:

```rust
enum PrivacyMode {
    Safe,
    Diagnostics,
    Full,
}
```

Safe mode is default.

## Units

Use strong typed units.

Internally store:

* bytes as `u64`
* percentages as `f32`
* durations as `Duration`
* timestamps as `SystemTime`
* frequencies as Hz
* temperatures as Celsius
* power as watts
* voltage as volts
* fan speed as RPM

Recommended dependency:

* `uom` for type-safe units (10M downloads, no_std compatible). Prevents unit confusion between Hz/MHz/GHz, Celsius/Fahrenheit, Watts/Volts, etc.

If `uom` is too heavy, use simple typed wrappers:

```rust
struct Bytes(u64);
struct Hertz(u64);
struct Celsius(f32);
struct Watts(f32);
```

## Sync vs async

Default API should be synchronous.

Reason: most OS data collection is local file/API reads and simple enough.

Optional async feature:

```toml
features = {
  "tokio": [],
  "stream": ["tokio"],
}
```

Async should only be used for:

* streaming samples
* long-running collectors
* WebSocket/dashboard integration
* async command providers

## Command execution

Some data may require external commands.

Use `duct`, not raw `std::process::Command` everywhere.

Command-based providers must be optional.

Examples:

* `smartctl` for SMART disk health
* `system_profiler` for macOS hardware info
* `powermetrics` for macOS power/thermal data
* `ioreg` for macOS I/O registry
* `nvidia-smi` fallback for NVIDIA GPU
* `lsblk` for Linux block devices
* `lspci` for PCI device listing
* `wmic` fallback only if needed on Windows

### duct usage patterns

Basic command (no shell, args as arrays):
```rust
use duct::cmd;
let output = cmd!("smartctl", "--json", "-a", "/dev/sda").read()?;
```

With timeout (always timeout external commands):
```rust
use duct::cmd;
use std::time::Duration;

let handle = cmd!("system_profiler", "SPHardwareDataType").start()?;
match handle.wait_timeout(Duration::from_secs(5))? {
    Some(output) => { /* process output */ }
    None => { handle.kill()?; } // timed out
}
```

Check command availability first:
```rust
use which::which;
if which("nvidia-smi").is_ok() {
    // nvidia-smi is available
}
```

Capture stderr for error reporting:
```rust
let output = cmd!("nvidia-smi", "--query-gpu=name", "--format=csv,noheader")
    .stderr_capture()
    .unchecked()
    .run()?;
if !output.status.success() {
    // report error from output.stderr
}
```

Rules:

* no shell strings — always use `cmd!(...)` with args as arrays
* timeout every command via `Handle::wait_timeout(Duration)`
* capture stderr via `.stderr_capture()`
* use `.unchecked()` when non-zero exit is expected (e.g., probing availability)
* check command exists with `which::which()` before invoking
* parse structured JSON where available
* redact command output before logging
* command providers disabled by default unless explicitly enabled
* gate all command providers behind `commands` feature flag

## Main public data model

### `SystemSnapshot`

```rust
struct SystemSnapshot {
    collected_at: SystemTime,
    static_info: StaticInfo,
    cpu: CpuSnapshot,
    memory: MemorySnapshot,
    storage: StorageSnapshot,
    disk_io: DiskIoSnapshot,
    network: NetworkSnapshot,
    processes: ProcessSnapshot,
    gpu: GpuSnapshot,
    battery: BatterySnapshot,
    sensors: SensorSnapshot,
    virtualization: VirtualizationSnapshot,
    capabilities: Capabilities,
    warnings: Vec<Finding>,
}
```

### `StaticInfo`

```rust
struct StaticInfo {
    os: OsInfo,
    kernel: KernelInfo,
    host: HostInfo,
    cpu: CpuStatic,
    memory: MemoryStatic,
    hardware: HardwareInventory,
}
```

### `SystemDelta`

```rust
struct SystemDelta {
    from: SystemTime,
    to: SystemTime,
    cpu: CpuDelta,
    disk_io: DiskIoDelta,
    network: NetworkDelta,
    processes: ProcessDelta,
    gpu: GpuDelta,
}
```

## Task-manager features

The crate should provide helpers for building a task manager UI, but not the UI itself.

Helpers:

* sorted process table
* top CPU processes
* top memory processes
* top disk I/O processes
* process tree
* per-core CPU chart data
* memory chart data
* network chart data
* disk chart data
* GPU chart data
* compact summary cards

Example:

```rust
let table = snapshot.processes
    .table()
    .sort_by_cpu_desc()
    .limit(20);
```

## Doctor module

Add a `doctor` module like the SSH/fail2ban plans.

Doctor checks:

### Provider checks

* `sysinfo` working
* OS info working
* disk provider working
* network provider working
* process provider working
* GPU provider availability
* battery provider availability
* sensors provider availability

### Permission checks

* can read process executable paths
* can read process command lines
* can read disk stats
* can read network stats
* can read DMI/SMBIOS
* can read SMART data
* can access NVML
* can access sensors
* running as admin/root when required

### Data quality checks

* CPU usage sampled with enough interval
* disk/network counters are monotonic
* counter wrap detected
* system clock sane
* duplicate disks removed
* virtual filesystems filtered
* container limits detected
* GPU identity without GPU metrics warning
* sensors unavailable warning

### Privacy checks

* serials redacted
* MAC addresses redacted
* process command args redacted
* public IP disabled
* environment collection disabled

## Presets

Expose collection presets.

```rust
enum Preset {
    Minimal,
    TaskManager,
    Diagnostics,
    HardwareInventory,
    ServerMonitoring,
    PrivacySafeBugReport,
}
```

### Minimal

* OS
* CPU usage
* memory usage
* disk usage
* network usage

### TaskManager

* CPU
* memory
* processes
* disks
* network
* GPU if available

### Diagnostics

* everything except secrets
* redacted identifiers
* doctor warnings

### HardwareInventory

* static hardware
* CPU specs
* memory specs
* storage devices
* PCI devices
* GPU identity
* OS version

### ServerMonitoring

* CPU
* memory
* disk
* disk I/O
* network I/O
* processes
* cgroups
* uptime
* services optional later

### PrivacySafeBugReport

* OS
* architecture
* CPU family
* memory total
* GPU model
* driver versions
* no serials
* no MACs
* no hostname unless enabled
* no process command lines

## Feature flags

Recommended:

```toml
default = ["sysinfo-provider", "os-info", "serde"]

sysinfo-provider = ["sysinfo"]
linux-procfs = ["procfs"]
linux-sensors = ["lm-sensors"]
linux-udev = ["udev"]
linux-rtnetlink = ["rtnetlink"]
linux-cgroups = ["cgroups-rs"]

os-info = ["os_info"]
cpu-cpuid = ["raw-cpuid"]
hardware-dmi = ["dmidecode"]
hardware-pci = ["pci-info", "pci-ids"]
hardware-topology = ["hwlocality"]

gpu-wgpu = ["wgpu"]
gpu-nvidia = ["nvml-wrapper"]

battery = ["starship-battery"]
smart = []
commands = ["duct", "which"]
serde = ["dep:serde"]
tokio = ["dep:tokio"]
```

Keep heavy platform crates optional.

## Platform support targets

### Linux

Strongest support.

Expected:

* CPU usage/specs
* memory/swap
* processes
* disk usage
* disk I/O
* network interfaces
* network I/O
* sensors
* battery
* cgroups
* Docker/container detection
* DMI/SMBIOS
* PCI devices
* NVIDIA GPU via NVML
* SMART via optional provider

### macOS

Good support, but GPU utilization and sensors are harder.

Expected:

* OS info
* CPU usage
* memory
* processes
* disk usage
* network
* battery
* GPU identity
* Apple Silicon static hardware info where possible

Optional command providers:

* `system_profiler`
* `ioreg`
* `powermetrics`

### Windows

Good support with WMI/windows APIs.

Expected:

* OS info
* CPU usage
* memory
* processes
* disk usage
* network
* battery
* GPU identity
* NVIDIA GPU via NVML
* hardware info through WMI

Use `wmi` and `windows` crates for deeper Windows-only providers.

## Avoid these mistakes

Do not:

* return fake zeroes for unsupported metrics
* block for multiple seconds in a normal snapshot
* call heavy commands every refresh
* expose secrets from process command lines
* expose serial numbers by default
* make GPU metrics look equally supported everywhere
* use shell strings
* make async mandatory
* make Linux-only features mandatory
* assume container memory limit equals host memory
* assume task manager rates can be calculated from one sample
* assume disk usage and disk I/O are the same thing
* assume filesystem mount and physical disk are the same thing
* assume CPU frequency is reliable on all platforms

## MVP scope

MVP should include:

* `SysProbe::new()`
* `snapshot()`
* `Collector`
* `SystemSnapshot`
* `SystemDelta`
* CPU usage
* per-core CPU usage
* CPU brand/frequency/core counts
* memory usage
* swap usage
* OS info
* kernel version
* hostname redacted option
* uptime
* disk usage
* disk I/O where available
* network interfaces
* network I/O
* process list
* top CPU/memory processes
* NVIDIA GPU metrics if NVML available
* generic GPU identity if available
* battery if available
* sensors if available
* capability report
* doctor report
* serde support
* fake provider tests
* Linux/macOS/Windows CI where possible

Do not include SMART, DMI, PCI inventory, WMI deep queries, or cgroups in the MVP unless time allows.

## v1.1 scope

Add:

* SMBIOS/DMI hardware inventory
* memory DIMM info
* PCI device inventory
* SMART disk health
* cgroup/container limits
* process tree helpers
* active connections
* listening ports
* service status optional
* GPU provider expansion
* privacy-safe bug report generator
* Prometheus export model
* JSON report model

## Testing plan

### Unit tests

* data model serialization
* unit conversions
* delta calculation
* counter wrap
* unsupported values
* privacy redaction
* process sorting
* provider error mapping
* capability merging

### Fixture tests

Use captured fixtures for:

* Linux `/proc/stat`
* Linux `/proc/meminfo`
* Linux `/proc/diskstats`
* Linux `/proc/net/dev`
* Linux `/sys/class`
* WMI output samples
* macOS command output samples
* NVML mock output
* SMART JSON output

### Integration tests

Run on:

* Linux
* macOS
* Windows

Test:

* snapshot does not panic
* CPU/memory available
* disk list non-empty
* network list non-empty
* process list non-empty
* privacy redaction works
* unsupported GPU does not fail whole snapshot

### Performance tests

Measure:

* full snapshot time
* task-manager refresh loop overhead
* process list scan time
* disk/network delta calculation
* GPU provider timeout behavior

## Public examples

Ship these examples:

```text
examples/
  simple_snapshot.rs
  task_manager_loop.rs
  top_processes.rs
  hardware_inventory.rs
  gpu_status.rs
  network_rates.rs
  disk_rates.rs
  privacy_safe_report.rs
  doctor.rs
```

## Documentation deliverables

README should include:

* library-first positioning
* supported platforms
* feature flag table
* quickstart
* snapshot example
* delta example
* task-manager example
* privacy model
* provider model
* unsupported metric behavior
* GPU limitations
* Linux/macOS/Windows notes
* container/cgroup notes
* performance notes

## Implementation order

### Sprint 1: Core model

* crate skeleton
* error types
* units
* snapshot model
* delta model
* capabilities model
* privacy model

### Sprint 2: Base provider

* `sysinfo` provider
* OS info provider
* CPU snapshot
* memory snapshot
* disk usage
* network counters
* process list

### Sprint 3: Collector and deltas

* refresh interval
* CPU deltas
* disk I/O deltas
* network I/O deltas
* process CPU deltas
* top-process helpers

### Sprint 4: Hardware/specs

* CPU specs
* OS/kernel info
* hardware inventory base
* DMI optional provider
* PCI optional provider
* topology optional provider

### Sprint 5: GPU/battery/sensors

* GPU identity
* NVIDIA NVML provider
* battery provider
* sensor provider
* capability warnings

### Sprint 6: Doctor and docs

* doctor report
* privacy-safe report
* examples
* CI
* integration tests
* documentation

## Final v1 audit checklist

Before v1 is done, the crate should support:

* CPU usage
* per-core CPU usage
* CPU specs
* memory usage
* swap usage
* OS info
* OS version
* kernel version
* uptime
* disk usage
* disk I/O
* network interface info
* network I/O
* process list
* top CPU processes
* top memory processes
* process tree helper
* GPU identity
* NVIDIA GPU metrics when available
* battery status
* temperature sensors where available
* static hardware summary
* capability report
* doctor report
* privacy redaction
* serde output
* fake provider testing
* cross-platform compilation
* no shell-string execution by default
* no fake zero values for unsupported metrics
* clear unsupported/permission-denied errors
