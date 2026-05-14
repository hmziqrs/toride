use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct Wireguard;

#[async_trait]
impl SetupModule for Wireguard {
    fn id(&self) -> ModuleId { ModuleId::Wireguard }
    fn name(&self) -> &'static str { "WireGuard" }
    fn description(&self) -> &'static str { "Fast, modern VPN tunnel" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::Networking }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        Ok(vec![
            InstallAction::AptInstall {
                packages: vec!["wireguard".into(), "wireguard-tools".into()],
            },
        ])
    }

    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> ModuleResult<ApplyOutcome> {
        let actions = self.plan(ctx).await?;
        crate::executor::command::execute_actions(&actions, &tx, ctx.is_dry_run).await
    }

    async fn verify(&self, _ctx: &Context) -> ModuleResult<VerifyResult> {
        if which::which("wg").is_ok() {
            Ok(VerifyResult::Installed)
        } else {
            Ok(VerifyResult::NotInstalled)
        }
    }
}
