use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct Swap;

#[async_trait]
impl SetupModule for Swap {
    fn id(&self) -> ModuleId { ModuleId::Swap }
    fn name(&self) -> &'static str { "Swap" }
    fn description(&self) -> &'static str { "Create and enable a swap file" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::SystemBasics }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        if std::path::Path::new("/swapfile").exists() {
            return Ok(PreflightResult::Warning("Swap file already exists".into()));
        }
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        Ok(vec![
            InstallAction::Exec {
                cmd: "fallocate".into(),
                args: vec!["-l".into(), "2G".into(), "/swapfile".into()],
                env: vec![],
                as_user: None,
            },
            InstallAction::Exec {
                cmd: "chmod".into(),
                args: vec!["600".into(), "/swapfile".into()],
                env: vec![],
                as_user: None,
            },
            InstallAction::Exec {
                cmd: "mkswap".into(),
                args: vec!["/swapfile".into()],
                env: vec![],
                as_user: None,
            },
            InstallAction::Exec {
                cmd: "swapon".into(),
                args: vec!["/swapfile".into()],
                env: vec![],
                as_user: None,
            },
            InstallAction::AppendLine {
                path: "/etc/fstab".into(),
                line: "/swapfile none swap sw 0 0".into(),
                marker: "toride-swap".into(),
            },
        ])
    }

    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> ModuleResult<ApplyOutcome> {
        let actions = self.plan(ctx).await?;
        crate::executor::command::execute_actions(&actions, &tx, ctx.is_dry_run).await
    }

    async fn verify(&self, _ctx: &Context) -> ModuleResult<VerifyResult> {
        let output = tokio::process::Command::new("swapon")
            .args(["--show=NAME", "--noheadings"])
            .output()
            .await
            .map_err(|e| ModuleError::Exec(e.to_string()))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.lines().any(|l| l.trim() == "/swapfile") {
            Ok(VerifyResult::Installed)
        } else {
            Ok(VerifyResult::NotInstalled)
        }
    }
}
