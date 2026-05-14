use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct Traefik;

#[async_trait]
impl SetupModule for Traefik {
    fn id(&self) -> ModuleId { ModuleId::Traefik }
    fn name(&self) -> &'static str { "Traefik" }
    fn description(&self) -> &'static str { "Cloud-native reverse proxy with auto-discovery" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![ModuleId::Caddy, ModuleId::Nginx] }
    fn category(&self) -> Category { Category::ReverseProxy }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        if which::which("traefik").is_ok() {
            return Ok(PreflightResult::Warning("Traefik is already installed".into()));
        }
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        let arch = std::env::consts::ARCH;
        let traefik_arch = match arch {
            "x86_64" => "amd64",
            "aarch64" => "arm64",
            other => other,
        };
        let url = format!(
            "https://github.com/traefik/traefik/releases/download/v3.3.6/traefik_v3.3.6_linux_{}.tar.gz",
            traefik_arch
        );

        Ok(vec![
            InstallAction::Exec {
                cmd: "bash".into(),
                args: vec!["-c".into(), format!("curl -sL {} | tar xz -C /tmp && mv /tmp/traefik /usr/local/bin/traefik && chmod +x /usr/local/bin/traefik", url)],
                env: vec![],
                as_user: None,
            },
            InstallAction::WriteFile {
                path: "/etc/systemd/system/traefik.service".into(),
                content: "[Unit]\nDescription=Traefik Reverse Proxy\nAfter=network.target\n\n[Service]\nExecStart=/usr/local/bin/traefik --configFile=/etc/traefik/traefik.yml\nRestart=always\nRestartSec=5\n\n[Install]\nWantedBy=multi-user.target\n".into(),
                mode: 0o644,
                backup: false,
            },
            InstallAction::Exec {
                cmd: "mkdir".into(),
                args: vec!["-p".into(), "/etc/traefik".into()],
                env: vec![],
                as_user: None,
            },
            InstallAction::Systemctl {
                unit: "traefik".into(),
                op: "enable".into(),
            },
            InstallAction::Systemctl {
                unit: "traefik".into(),
                op: "start".into(),
            },
        ])
    }

    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> ModuleResult<ApplyOutcome> {
        let actions = self.plan(ctx).await?;
        crate::executor::command::execute_actions(&actions, &tx, ctx.is_dry_run).await
    }

    async fn verify(&self, _ctx: &Context) -> ModuleResult<VerifyResult> {
        if which::which("traefik").is_ok() {
            Ok(VerifyResult::Installed)
        } else {
            Ok(VerifyResult::NotInstalled)
        }
    }
}
