use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct Dokploy;

#[async_trait]
impl SetupModule for Dokploy {
    fn id(&self) -> ModuleId { ModuleId::Dokploy }
    fn name(&self) -> &'static str { "Dokploy" }
    fn description(&self) -> &'static str { "Self-hosted PaaS (requires Docker)" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![ModuleId::Docker] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![ModuleId::Coolify] }
    fn category(&self) -> Category { Category::ServerManagers }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        Ok(vec![
            InstallAction::Exec {
                cmd: "docker".into(),
                args: vec![
                    "run".into(), "-d".into(),
                    "--name".into(), "dokploy".into(),
                    "--restart".into(), "always".into(),
                    "-v".into(), "/var/run/docker.sock:/var/run/docker.sock".into(),
                    "-v".into(), "/etc/dokploy:/etc/dokploy".into(),
                    "-p".into(), "3000:3000".into(),
                    "dokploy/dokploy:latest".into(),
                ],
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
