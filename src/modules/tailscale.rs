use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct Tailscale;

#[async_trait]
impl SetupModule for Tailscale {
    fn id(&self) -> ModuleId { ModuleId::Tailscale }
    fn name(&self) -> &'static str { "Tailscale" }
    fn description(&self) -> &'static str { "Mesh VPN with zero-config WireGuard" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::Networking }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        if which::which("tailscale").is_ok() {
            return Ok(PreflightResult::Warning("Tailscale is already installed".into()));
        }
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        Ok(vec![
            InstallAction::DownloadScript {
                url: "https://tailscale.com/install.sh".into(),
                sha256: String::new(),
                run_as: "root".into(),
                env: vec![],
            },
            InstallAction::Systemctl {
                unit: "tailscaled".into(),
                op: "enable".into(),
            },
            InstallAction::Systemctl {
                unit: "tailscaled".into(),
                op: "start".into(),
            },
        ])
    }

    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> ModuleResult<ApplyOutcome> {
        let actions = self.plan(ctx).await?;
        crate::executor::command::execute_actions(&actions, &tx, ctx.is_dry_run).await
    }

    async fn verify(&self, _ctx: &Context) -> ModuleResult<VerifyResult> {
        if which::which("tailscale").is_ok() {
            Ok(VerifyResult::Installed)
        } else {
            Ok(VerifyResult::NotInstalled)
        }
    }
}
