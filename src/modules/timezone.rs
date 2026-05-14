use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct Timezone;

#[async_trait]
impl SetupModule for Timezone {
    fn id(&self) -> ModuleId { ModuleId::Timezone }
    fn name(&self) -> &'static str { "Timezone" }
    fn description(&self) -> &'static str { "Set system timezone and enable NTP sync" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::SystemBasics }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        Ok(vec![
            InstallAction::Exec {
                cmd: "timedatectl".into(),
                args: vec!["set-timezone".into(), "UTC".into()],
                env: vec![],
                as_user: None,
            },
            InstallAction::Exec {
                cmd: "timedatectl".into(),
                args: vec!["set-ntp".into(), "true".into()],
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
