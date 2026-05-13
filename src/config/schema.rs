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
    pub auto_security_updates: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

fn default_true() -> bool { true }
fn default_swap_size() -> String { "2G".into() }

impl Default for UserConfig { fn default() -> Self { Self { name: String::new(), ssh_key_path: String::new(), passwordless_sudo: true } } }
impl Default for SecurityConfig { fn default() -> Self { Self { disable_root_login: false, disable_password_login: false, ufw: true, auto_security_updates: false } } }
impl Default for RuntimesConfig { fn default() -> Self { Self { node: false, bun: false, deno: false, rust: false, go: false, python: false } } }
impl Default for ContainersConfig { fn default() -> Self { Self { docker: true, docker_log_rotation: true } } }
impl Default for SwapConfig { fn default() -> Self { Self { enabled: true, size: default_swap_size() } } }
