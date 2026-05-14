use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct Fail2Ban;

#[async_trait]
impl SetupModule for Fail2Ban {
    fn id(&self) -> ModuleId { ModuleId::Fail2Ban }
    fn name(&self) -> &'static str { "Fail2Ban" }
    fn description(&self) -> &'static str { "Intrusion prevention with systemd backend for SSH" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![ModuleId::SystemUpdate] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::FirewallAndSecurity }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        if which::which("fail2ban-client").is_ok() {
            return Ok(PreflightResult::Warning("Fail2Ban is already installed".into()));
        }
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        Ok(vec![
            InstallAction::AptInstall {
                packages: vec!["fail2ban".into(), "python3-systemd".into()],
            },
            InstallAction::WriteFile {
                path: "/etc/fail2ban/jail.local".into(),
                content: "[DEFAULT]\nbackend = systemd\nbanaction = nftables-multiport\n\n[sshd]\nenabled = true\nport = ssh\nfilter = sshd\nlogpath = /var/log/auth.log\nmaxretry = 5\nbantime = 3600\nfindtime = 600\n".into(),
                mode: 0o644,
                backup: true,
            },
            InstallAction::Systemctl {
                unit: "fail2ban".into(),
                op: "enable".into(),
            },
            InstallAction::Systemctl {
                unit: "fail2ban".into(),
                op: "start".into(),
            },
        ])
    }

    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> ModuleResult<ApplyOutcome> {
        let actions = self.plan(ctx).await?;
        crate::executor::command::execute_actions(&actions, &tx, ctx.is_dry_run).await
    }

    async fn verify(&self, _ctx: &Context) -> ModuleResult<VerifyResult> {
        if which::which("fail2ban-client").is_ok() {
            Ok(VerifyResult::Installed)
        } else {
            Ok(VerifyResult::NotInstalled)
        }
    }
}
