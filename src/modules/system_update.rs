use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct SystemUpdate;

#[async_trait]
impl SetupModule for SystemUpdate {
    fn id(&self) -> ModuleId { ModuleId::SystemUpdate }
    fn name(&self) -> &'static str { "System Update" }
    fn description(&self) -> &'static str { "Update apt package index and upgrade installed packages" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::SystemBasics }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        Ok(vec![
            InstallAction::Exec {
                cmd: "apt-get".into(),
                args: vec!["update".into()],
                env: vec![],
                as_user: None,
            },
            InstallAction::Exec {
                cmd: "apt-get".into(),
                args: vec!["upgrade".into(), "-y".into()],
                env: vec![],
                as_user: None,
            },
            InstallAction::AptInstall {
                packages: vec![
                    "curl".into(), "wget".into(), "git".into(), "unzip".into(),
                    "jq".into(), "ca-certificates".into(), "gnupg".into(),
                    "build-essential".into(), "pkg-config".into(),
                ],
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
