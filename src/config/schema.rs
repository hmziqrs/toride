use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub profile: String,
    #[serde(default)]
    pub user: UserConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub runtimes: RuntimesConfig,
    #[serde(default)]
    pub containers: ContainersConfig,
    #[serde(default)]
    pub swap: SwapConfig,
    #[serde(default)]
    pub networking: NetworkingConfig,
    #[serde(default)]
    pub server_manager: ServerManagerConfig,
    #[serde(default)]
    pub reverse_proxy: ReverseProxyConfig,
    #[serde(default)]
    pub backup: BackupConfig,
    #[serde(default)]
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub ssh_key_path: String,
    #[serde(default = "default_true")]
    pub passwordless_sudo: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default)]
    pub disable_root_login: bool,
    #[serde(default)]
    pub disable_password_login: bool,
    #[serde(default = "default_true")]
    pub ufw: bool,
    #[serde(default)]
    pub fail2ban: bool,
    #[serde(default)]
    pub cloudflare_only_http: bool,
    #[serde(default)]
    pub auto_security_updates: bool,
    #[serde(default)]
    pub kernel_hardening: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct RuntimesConfig {
    #[serde(default)]
    pub node: bool,
    #[serde(default)]
    pub bun: bool,
    #[serde(default)]
    pub deno: bool,
    #[serde(default)]
    pub rust: bool,
    #[serde(default)]
    pub go: bool,
    #[serde(default)]
    pub python: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainersConfig {
    #[serde(default = "default_true")]
    pub docker: bool,
    #[serde(default = "default_true")]
    pub docker_log_rotation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_swap_size")]
    pub size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkingConfig {
    #[serde(default)]
    pub tailscale: bool,
    #[serde(default)]
    pub cloudflare_tunnel: bool,
    #[serde(default)]
    pub wireguard: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerManagerConfig {
    #[serde(default)]
    pub manager: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReverseProxyConfig {
    #[serde(default)]
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    #[serde(default)]
    pub restic: bool,
    #[serde(default)]
    pub borg: bool,
    #[serde(default)]
    pub rclone: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    #[serde(default)]
    pub node_exporter: bool,
    #[serde(default)]
    pub uptime_kuma: bool,
    #[serde(default)]
    pub netdata: bool,
    #[serde(default)]
    pub prometheus: bool,
    #[serde(default)]
    pub grafana: bool,
}

fn default_true() -> bool { true }
fn default_swap_size() -> String { "2G".into() }

impl Default for UserConfig { fn default() -> Self { Self { name: String::new(), ssh_key_path: String::new(), passwordless_sudo: true } } }
impl Default for SecurityConfig { fn default() -> Self { Self { disable_root_login: false, disable_password_login: false, ufw: true, fail2ban: false, cloudflare_only_http: false, auto_security_updates: false, kernel_hardening: false } } }
impl Default for ContainersConfig { fn default() -> Self { Self { docker: true, docker_log_rotation: true } } }
impl Default for SwapConfig { fn default() -> Self { Self { enabled: true, size: default_swap_size() } } }
impl Default for NetworkingConfig { fn default() -> Self { Self { tailscale: false, cloudflare_tunnel: false, wireguard: false } } }
impl Default for ServerManagerConfig { fn default() -> Self { Self { manager: String::new() } } }
impl Default for ReverseProxyConfig { fn default() -> Self { Self { mode: String::new() } } }
impl Default for BackupConfig { fn default() -> Self { Self { restic: false, borg: false, rclone: false } } }
impl Default for MonitoringConfig { fn default() -> Self { Self { node_exporter: false, uptime_kuma: false, netdata: false, prometheus: false, grafana: false } } }
