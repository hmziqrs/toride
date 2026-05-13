use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct Ufw;

#[async_trait]
impl SetupModule for Ufw {
    fn id(&self) -> ModuleId { ModuleId::Ufw }
    fn name(&self) -> &'static str { "UFW Firewall" }
    fn description(&self) -> &'static str { "Install and configure UFW firewall with SSH allow" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::FirewallAndSecurity }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        Ok(vec![
            InstallAction::AptInstall {
                packages: vec!["ufw".into()],
            },
            InstallAction::Exec {
                cmd: "ufw".into(),
                args: vec!["default".into(), "deny".into(), "incoming".into()],
                env: vec![],
                as_user: None,
            },
            InstallAction::Exec {
                cmd: "ufw".into(),
                args: vec!["default".into(), "allow".into(), "outgoing".into()],
                env: vec![],
                as_user: None,
            },
            InstallAction::Exec {
                cmd: "ufw".into(),
                args: vec!["allow".into(), "OpenSSH".into()],
                env: vec![],
                as_user: None,
            },
            InstallAction::Exec {
                cmd: "ufw".into(),
                args: vec!["--force".into(), "enable".into()],
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
