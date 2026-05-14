pub mod docker;
pub mod mise;
pub mod swap;
pub mod system_update;
pub mod ufw;
pub mod user_ssh;

pub mod fail2ban;
pub mod unattended_upgrades;
pub mod tailscale;
pub mod cloudflare_http;
pub mod sysctl;
pub mod hostname;
pub mod timezone;
pub mod dokploy;
pub mod coolify;
pub mod caddy;
pub mod nginx;
pub mod traefik;

pub mod cloudflare_tunnel;
pub mod wireguard;
pub mod restic;
pub mod borg;
pub mod rclone;
pub mod node_exporter;
pub mod uptime_kuma;
pub mod netdata;
pub mod prometheus;
pub mod grafana;
pub mod db_dump;

use async_trait::async_trait;
use std::collections::BTreeMap;

use crate::tui::model::{Category, ModuleId};

#[derive(Debug, thiserror::Error)]
pub enum ModuleError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Exec(String),
}

pub type ModuleResult<T> = Result<T, ModuleError>;

#[derive(Debug, Clone)]
pub struct Context {
    pub is_dry_run: bool,
    pub is_test: bool,
    pub target_user: String,
    pub ssh_public_key: String,
}

#[derive(Debug)]
pub enum PreflightResult {
    Ok,
    Warning(String),
    Skip(String),
}

#[derive(Debug)]
pub enum ApplyOutcome {
    Changed,
    AlreadyApplied,
    Skipped,
}

#[derive(Debug)]
pub enum VerifyResult {
    Installed,
    NotInstalled,
    Partial(String),
}

pub type ProgressTx = tokio::sync::mpsc::UnboundedSender<crate::tui::model::ProgressEvent>;

#[async_trait]
pub trait SetupModule: Send + Sync {
    fn id(&self) -> ModuleId;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn dependencies(&self) -> Vec<ModuleId>;
    fn conflicts(&self) -> Vec<ModuleId>;
    fn category(&self) -> Category;

    async fn preflight(&self, ctx: &Context) -> ModuleResult<PreflightResult>;
    async fn plan(&self, ctx: &Context) -> ModuleResult<Vec<InstallAction>>;
    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> ModuleResult<ApplyOutcome>;
    async fn verify(&self, ctx: &Context) -> ModuleResult<VerifyResult>;
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum InstallAction {
    AptInstall { packages: Vec<String> },
    AptRepoAdd { name: String, key_url: String, sources_line: String, sha256: String },
    WriteFile { path: String, content: String, mode: u32, backup: bool },
    AppendLine { path: String, line: String, marker: String },
    Systemctl { unit: String, op: String },
    UfwRule { rule: String },
    UserCreate { name: String, groups: Vec<String>, shell: String },
    UserAddKey { user: String, key: String },
    DownloadScript { url: String, sha256: String, run_as: String, env: Vec<(String, String)> },
    Exec { cmd: String, args: Vec<String>, env: Vec<(String, String)>, as_user: Option<String> },
    DnfInstall { packages: Vec<String> },
    DnfRepoAdd { name: String, baseurl: String, gpgkey: String },
}

impl InstallAction {
    pub fn to_shell_preview(&self) -> String {
        match self {
            Self::AptInstall { packages } => format!("apt install -y {}", packages.join(" ")),
            Self::AptRepoAdd { name, key_url, sources_line, .. } => {
                format!("add-apt-repo {} (key: {}, sources: {})", name, key_url, sources_line)
            }
            Self::WriteFile { path, mode, backup, .. } => {
                let bak = if *backup { " [backup]" } else { "" };
                format!("write {} (mode: {:o}){}", path, mode, bak)
            }
            Self::AppendLine { path, marker, .. } => {
                format!("append to {} [marker: {}]", path, marker)
            }
            Self::Systemctl { unit, op } => format!("systemctl {} {}", op.to_lowercase(), unit),
            Self::UfwRule { rule } => format!("ufw {}", rule),
            Self::UserCreate { name, groups, shell } => {
                format!("useradd {} -G {} -s {}", name, groups.join(","), shell)
            }
            Self::UserAddKey { user, .. } => format!("add SSH key for {}", user),
            Self::DownloadScript { url, sha256, .. } => {
                format!("download {} (sha256: {}…)", url, &sha256[..16.min(sha256.len())])
            }
            Self::Exec { cmd, args, as_user, .. } => {
                let user_prefix = as_user.as_ref().map(|u| format!("{}: ", u)).unwrap_or_default();
                format!("{}{} {}", user_prefix, cmd, args.join(" "))
            }
            Self::DnfInstall { packages } => format!("dnf install -y {}", packages.join(" ")),
            Self::DnfRepoAdd { name, baseurl, .. } => {
                format!("add-dnf-repo {} ({})", name, baseurl)
            }
        }
    }
}

pub fn registry() -> BTreeMap<ModuleId, Box<dyn SetupModule>> {
    let mut reg: BTreeMap<ModuleId, Box<dyn SetupModule>> = BTreeMap::new();
    reg.insert(ModuleId::SystemUpdate, Box::new(system_update::SystemUpdate));
    reg.insert(ModuleId::Swap, Box::new(swap::Swap));
    reg.insert(ModuleId::UserSsh, Box::new(user_ssh::UserSsh));
    reg.insert(ModuleId::Ufw, Box::new(ufw::Ufw));
    reg.insert(ModuleId::Docker, Box::new(docker::Docker));
    reg.insert(ModuleId::Mise, Box::new(mise::Mise));
    reg.insert(ModuleId::Fail2Ban, Box::new(fail2ban::Fail2Ban));
    reg.insert(ModuleId::UnattendedUpgrades, Box::new(unattended_upgrades::UnattendedUpgrades));
    reg.insert(ModuleId::Tailscale, Box::new(tailscale::Tailscale));
    reg.insert(ModuleId::CloudflareHttp, Box::new(cloudflare_http::CloudflareHttp));
    reg.insert(ModuleId::SysctlHardening, Box::new(sysctl::SysctlHardening));
    reg.insert(ModuleId::Hostname, Box::new(hostname::Hostname));
    reg.insert(ModuleId::Timezone, Box::new(timezone::Timezone));
    reg.insert(ModuleId::Dokploy, Box::new(dokploy::Dokploy));
    reg.insert(ModuleId::Coolify, Box::new(coolify::Coolify));
    reg.insert(ModuleId::Caddy, Box::new(caddy::Caddy));
    reg.insert(ModuleId::Nginx, Box::new(nginx::Nginx));
    reg.insert(ModuleId::Traefik, Box::new(traefik::Traefik));
    reg.insert(ModuleId::CloudflareTunnel, Box::new(cloudflare_tunnel::CloudflareTunnel));
    reg.insert(ModuleId::Wireguard, Box::new(wireguard::Wireguard));
    reg.insert(ModuleId::Restic, Box::new(restic::Restic));
    reg.insert(ModuleId::Borg, Box::new(borg::Borg));
    reg.insert(ModuleId::Rclone, Box::new(rclone::Rclone));
    reg.insert(ModuleId::NodeExporter, Box::new(node_exporter::NodeExporter));
    reg.insert(ModuleId::UptimeKuma, Box::new(uptime_kuma::UptimeKuma));
    reg.insert(ModuleId::Netdata, Box::new(netdata::Netdata));
    reg.insert(ModuleId::Prometheus, Box::new(prometheus::Prometheus));
    reg.insert(ModuleId::Grafana, Box::new(grafana::Grafana));
    reg.insert(ModuleId::DbDump, Box::new(db_dump::DbDump));
    reg
}
