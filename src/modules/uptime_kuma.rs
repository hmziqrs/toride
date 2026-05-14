use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct UptimeKuma;

#[async_trait]
impl SetupModule for UptimeKuma {
    fn id(&self) -> ModuleId { ModuleId::UptimeKuma }
    fn name(&self) -> &'static str { "Uptime Kuma" }
    fn description(&self) -> &'static str { "Self-hosted monitoring tool (requires Docker)" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![ModuleId::Docker] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::Monitoring }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        Ok(vec![
            InstallAction::Exec {
                cmd: "docker".into(),
                args: vec![
                    "run".into(), "-d".into(),
                    "--name".into(), "uptime-kuma".into(),
                    "--restart".into(), "always".into(),
                    "-p".into(), "3001:3001".into(),
                    "-v".into(), "uptime-kuma:/app/data".into(),
                    "louislam/uptime-kuma:latest".into(),
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
