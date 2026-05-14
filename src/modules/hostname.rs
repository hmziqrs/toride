use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct Hostname;

#[async_trait]
impl SetupModule for Hostname {
    fn id(&self) -> ModuleId { ModuleId::Hostname }
    fn name(&self) -> &'static str { "Hostname" }
    fn description(&self) -> &'static str { "Set system hostname" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::SystemBasics }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        Ok(vec![
            InstallAction::Exec {
                cmd: "hostnamectl".into(),
                args: vec!["set-hostname".into(), "toride-server".into()],
                env: vec![],
                as_user: None,
            },
        ])
    }

    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> ModuleResult<ApplyOutcome> {
        let actions = self.plan(ctx).await?;
        crate::executor::command::execute_actions(&actions, &tx, ctx.is_dry_run).await
    }

    async fn verify(&self, _ctx: &Context) -> ModuleResult<VerifyResult> {
        Ok(VerifyResult::Installed)
    }
}
