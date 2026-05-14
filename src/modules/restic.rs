use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct Restic;

#[async_trait]
impl SetupModule for Restic {
    fn id(&self) -> ModuleId { ModuleId::Restic }
    fn name(&self) -> &'static str { "Restic" }
    fn description(&self) -> &'static str { "Fast, secure, efficient backup program" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::Backup }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        if which::which("restic").is_ok() {
            return Ok(PreflightResult::Warning("Restic is already installed".into()));
        }
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        Ok(vec![
            InstallAction::AptInstall {
                packages: vec!["restic".into()],
            },
        ])
    }

    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> ModuleResult<ApplyOutcome> {
        let actions = self.plan(ctx).await?;
        crate::executor::command::execute_actions(&actions, &tx, ctx.is_dry_run).await
    }

    async fn verify(&self, _ctx: &Context) -> ModuleResult<VerifyResult> {
        if which::which("restic").is_ok() {
            Ok(VerifyResult::Installed)
        } else {
            Ok(VerifyResult::NotInstalled)
        }
    }
}
