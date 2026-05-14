use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct UnattendedUpgrades;

#[async_trait]
impl SetupModule for UnattendedUpgrades {
    fn id(&self) -> ModuleId { ModuleId::UnattendedUpgrades }
    fn name(&self) -> &'static str { "Auto Security Updates" }
    fn description(&self) -> &'static str { "Automatic installation of security updates via unattended-upgrades" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![ModuleId::SystemUpdate] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::SystemBasics }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        Ok(vec![
            InstallAction::AptInstall {
                packages: vec!["unattended-upgrades".into(), "apt-listchanges".into()],
            },
            InstallAction::WriteFile {
                path: "/etc/apt/apt.conf.d/50unattended-upgrades".into(),
                content: "Unattended-Upgrade::Allowed-Origins {\n    \"${distro_id}:${distro_codename}-security\";\n    \"${distro_id}ESMApps:${distro_codename}-apps-security\";\n    \"${distro_id}ESM:${distro_codename}-infra-security\";\n};\nUnattended-Upgrade::AutoFixInterruptedDpkg \"true\";\nUnattended-Upgrade::Remove-Unused-Dependencies \"true\";\nUnattended-Upgrade::Automatic-Reboot \"false\";\n".into(),
                mode: 0o644,
                backup: true,
            },
            InstallAction::WriteFile {
                path: "/etc/apt/apt.conf.d/20auto-upgrades".into(),
                content: "APT::Periodic::Update-Package-Lists \"1\";\nAPT::Periodic::Unattended-Upgrade \"1\";\nAPT::Periodic::Download-Upgradeable-Packages \"1\";\nAPT::Periodic::AutocleanInterval \"7\";\n".into(),
                mode: 0o644,
                backup: true,
            },
        ])
    }

    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> ModuleResult<ApplyOutcome> {
        let actions = self.plan(ctx).await?;
        crate::executor::command::execute_actions(&actions, &tx, ctx.is_dry_run).await
    }

    async fn verify(&self, _ctx: &Context) -> ModuleResult<VerifyResult> {
        if std::path::Path::new("/etc/apt/apt.conf.d/20auto-upgrades").exists() {
            Ok(VerifyResult::Installed)
        } else {
            Ok(VerifyResult::NotInstalled)
        }
    }
}
