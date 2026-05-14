use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct Coolify;

#[async_trait]
impl SetupModule for Coolify {
    fn id(&self) -> ModuleId { ModuleId::Coolify }
    fn name(&self) -> &'static str { "Coolify" }
    fn description(&self) -> &'static str { "Self-hostable Heroku/Vercel alternative (requires Docker)" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![ModuleId::Docker] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![ModuleId::Dokploy] }
    fn category(&self) -> Category { Category::ServerManagers }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        Ok(vec![
            InstallAction::Exec {
                cmd: "bash".into(),
                args: vec!["-c".into(), "docker run -d --name coolify --restart always -v /data/coolify:/data -v /var/run/docker.sock:/var/run/docker.sock -p 8000:8000 ghcr.io/coollabsio/coolify:latest".into()],
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
        Ok(VerifyResult::NotInstalled)
    }
}
