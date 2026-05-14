use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct Netdata;

#[async_trait]
impl SetupModule for Netdata {
    fn id(&self) -> ModuleId { ModuleId::Netdata }
    fn name(&self) -> &'static str { "Netdata" }
    fn description(&self) -> &'static str { "Real-time system health monitoring and performance dashboards" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::Monitoring }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        if which::which("netdata").is_ok() {
            return Ok(PreflightResult::Warning("Netdata is already installed".into()));
        }
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        Ok(vec![
            InstallAction::AptInstall {
                packages: vec!["netdata".into()],
            },
            InstallAction::Systemctl {
                unit: "netdata".into(),
                op: "enable".into(),
            },
            InstallAction::Systemctl {
                unit: "netdata".into(),
                op: "start".into(),
            },
        ])
    }

    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> ModuleResult<ApplyOutcome> {
        let actions = self.plan(ctx).await?;
        crate::executor::command::execute_actions(&actions, &tx, ctx.is_dry_run).await
    }

    async fn verify(&self, _ctx: &Context) -> ModuleResult<VerifyResult> {
        if which::which("netdata").is_ok() {
            Ok(VerifyResult::Installed)
        } else {
            Ok(VerifyResult::NotInstalled)
        }
    }
}
