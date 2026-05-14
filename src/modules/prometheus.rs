use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct Prometheus;

#[async_trait]
impl SetupModule for Prometheus {
    fn id(&self) -> ModuleId { ModuleId::Prometheus }
    fn name(&self) -> &'static str { "Prometheus" }
    fn description(&self) -> &'static str { "Time-series monitoring and alerting system" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::Monitoring }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        if which::which("prometheus").is_ok() {
            return Ok(PreflightResult::Warning("Prometheus is already installed".into()));
        }
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        Ok(vec![
            InstallAction::AptInstall {
                packages: vec!["prometheus".into()],
            },
            InstallAction::Systemctl {
                unit: "prometheus".into(),
                op: "enable".into(),
            },
            InstallAction::Systemctl {
                unit: "prometheus".into(),
                op: "start".into(),
            },
        ])
    }

    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> ModuleResult<ApplyOutcome> {
        let actions = self.plan(ctx).await?;
        crate::executor::command::execute_actions(&actions, &tx, ctx.is_dry_run).await
    }

    async fn verify(&self, _ctx: &Context) -> ModuleResult<VerifyResult> {
        if which::which("prometheus").is_ok() {
            Ok(VerifyResult::Installed)
        } else {
            Ok(VerifyResult::NotInstalled)
        }
    }
}
