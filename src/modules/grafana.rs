use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct Grafana;

#[async_trait]
impl SetupModule for Grafana {
    fn id(&self) -> ModuleId { ModuleId::Grafana }
    fn name(&self) -> &'static str { "Grafana" }
    fn description(&self) -> &'static str { "Observability and data visualization platform" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::Monitoring }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        Ok(vec![
            InstallAction::Exec {
                cmd: "bash".into(),
                args: vec!["-c".into(), "curl -fsSL https://packages.grafana.com/gpg.key | gpg --dearmor -o /usr/share/keyrings/grafana-keyring.gpg".into()],
                env: vec![],
                as_user: None,
            },
            InstallAction::WriteFile {
                path: "/etc/apt/sources.list.d/grafana.list".into(),
                content: "deb [signed-by=/usr/share/keyrings/grafana-keyring.gpg] https://packages.grafana.com/oss/deb stable main\n".into(),
                mode: 0o644,
                backup: false,
            },
            InstallAction::AptInstall {
                packages: vec!["grafana".into()],
            },
            InstallAction::Systemctl {
                unit: "grafana-server".into(),
                op: "enable".into(),
            },
            InstallAction::Systemctl {
                unit: "grafana-server".into(),
                op: "start".into(),
            },
        ])
    }

    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> ModuleResult<ApplyOutcome> {
        let actions = self.plan(ctx).await?;
        crate::executor::command::execute_actions(&actions, &tx, ctx.is_dry_run).await
    }

    async fn verify(&self, _ctx: &Context) -> ModuleResult<VerifyResult> {
        if which::which("grafana-server").is_ok() {
            Ok(VerifyResult::Installed)
        } else {
            Ok(VerifyResult::NotInstalled)
        }
    }
}
