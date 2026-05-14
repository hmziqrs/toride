use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct Caddy;

#[async_trait]
impl SetupModule for Caddy {
    fn id(&self) -> ModuleId { ModuleId::Caddy }
    fn name(&self) -> &'static str { "Caddy" }
    fn description(&self) -> &'static str { "Reverse proxy with automatic HTTPS" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![ModuleId::SystemUpdate] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![ModuleId::Nginx, ModuleId::Traefik] }
    fn category(&self) -> Category { Category::ReverseProxy }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        if which::which("caddy").is_ok() {
            return Ok(PreflightResult::Warning("Caddy is already installed".into()));
        }
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        Ok(vec![
            InstallAction::AptInstall {
                packages: vec!["debian-keyring".into(), "debian-archive-keyring".into(), "apt-transport-https".into()],
            },
            InstallAction::Exec {
                cmd: "bash".into(),
                args: vec!["-c".into(), "curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg".into()],
                env: vec![],
                as_user: None,
            },
            InstallAction::Exec {
                cmd: "bash".into(),
                args: vec!["-c".into(), "curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | tee /etc/apt/sources.list.d/caddy-stable.list".into()],
                env: vec![],
                as_user: None,
            },
            InstallAction::Exec {
                cmd: "apt".into(),
                args: vec!["update".into()],
                env: vec![],
                as_user: None,
            },
            InstallAction::AptInstall {
                packages: vec!["caddy".into()],
            },
            InstallAction::Systemctl {
                unit: "caddy".into(),
                op: "enable".into(),
            },
            InstallAction::Systemctl {
                unit: "caddy".into(),
                op: "start".into(),
            },
        ])
    }

    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> ModuleResult<ApplyOutcome> {
        let actions = self.plan(ctx).await?;
        crate::executor::command::execute_actions(&actions, &tx, ctx.is_dry_run).await
    }

    async fn verify(&self, _ctx: &Context) -> ModuleResult<VerifyResult> {
        if which::which("caddy").is_ok() {
            Ok(VerifyResult::Installed)
        } else {
            Ok(VerifyResult::NotInstalled)
        }
    }
}
