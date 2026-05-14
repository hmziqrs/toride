use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct Rclone;

#[async_trait]
impl SetupModule for Rclone {
    fn id(&self) -> ModuleId { ModuleId::Rclone }
    fn name(&self) -> &'static str { "Rclone" }
    fn description(&self) -> &'static str { "Sync files to cloud storage providers" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::Backup }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        if which::which("rclone").is_ok() {
            return Ok(PreflightResult::Warning("Rclone is already installed".into()));
        }
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        Ok(vec![
            InstallAction::AptInstall {
                packages: vec!["rclone".into()],
            },
        ])
    }

    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> ModuleResult<ApplyOutcome> {
        let actions = self.plan(ctx).await?;
        crate::executor::command::execute_actions(&actions, &tx, ctx.is_dry_run).await
    }

    async fn verify(&self, _ctx: &Context) -> ModuleResult<VerifyResult> {
        if which::which("rclone").is_ok() {
            Ok(VerifyResult::Installed)
        } else {
            Ok(VerifyResult::NotInstalled)
        }
    }
}
