use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct UserSsh;

#[async_trait]
impl SetupModule for UserSsh {
    fn id(&self) -> ModuleId { ModuleId::UserSsh }
    fn name(&self) -> &'static str { "Users & SSH" }
    fn description(&self) -> &'static str { "Create sudo user, add SSH key, harden sshd config" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::UsersAndSsh }

    async fn preflight(&self, ctx: &Context) -> ModuleResult<PreflightResult> {
        if ctx.target_user.is_empty() {
            return Ok(PreflightResult::Warning("No target user configured".into()));
        }
        if ctx.ssh_public_key.is_empty() {
            return Ok(PreflightResult::Warning("No SSH public key configured".into()));
        }
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        let user = &ctx.target_user;
        let mut actions = vec![
            InstallAction::UserCreate {
                name: user.clone(),
                groups: vec!["sudo".into()],
                shell: "/bin/bash".into(),
            },
            InstallAction::UserAddKey {
                user: user.clone(),
                key: ctx.ssh_public_key.clone(),
            },
            InstallAction::WriteFile {
                path: format!("/etc/sudoers.d/00-toride-{}", user),
                content: format!("{} ALL=(ALL) NOPASSWD:ALL\n", user),
                mode: 0o440,
                backup: false,
            },
        ];

        // sshd hardening drop-in
        actions.push(InstallAction::WriteFile {
            path: "/etc/ssh/sshd_config.d/00-toride.conf".into(),
            content: "PermitRootLogin no\nPasswordAuthentication no\nKbdInteractiveAuthentication no\n".into(),
            mode: 0o644,
            backup: true,
        });

        // cloud-init override
        actions.push(InstallAction::Exec {
            cmd: "rm".into(),
            args: vec!["-f".into(), "/etc/ssh/sshd_config.d/50-cloud-init.conf".into()],
            env: vec![],
            as_user: None,
        });

        // validate and reload
        actions.push(InstallAction::Exec {
            cmd: "sshd".into(),
            args: vec!["-t".into()],
            env: vec![],
            as_user: None,
        });
        actions.push(InstallAction::Systemctl {
            unit: "ssh".into(),
            op: "reload".into(),
        });

        Ok(actions)
    }

    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> ModuleResult<ApplyOutcome> {
        let actions = self.plan(ctx).await?;
        crate::executor::command::execute_actions(&actions, &tx, ctx.is_dry_run).await
    }

    async fn verify(&self, _ctx: &Context) -> ModuleResult<VerifyResult> {
        Ok(VerifyResult::Installed)
    }
}
