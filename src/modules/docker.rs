use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct Docker;

#[async_trait]
impl SetupModule for Docker {
    fn id(&self) -> ModuleId { ModuleId::Docker }
    fn name(&self) -> &'static str { "Docker" }
    fn description(&self) -> &'static str { "Docker Engine + Compose plugin + log rotation + user group" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::Containers }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        if which::which("docker").is_ok() {
            return Ok(PreflightResult::Warning("Docker is already installed".into()));
        }
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        let codename = get_codename();
        let arch = std::env::consts::ARCH;
        let deb_arch = match arch {
            "aarch64" => "arm64",
            "x86_64" => "amd64",
            other => other,
        };
        let key_url: String = "https://download.docker.com/linux/debian/gpg".into();
        let sources_line = format!("deb [arch={} signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian {} stable", deb_arch, codename);

        Ok(vec![
            InstallAction::Exec {
                cmd: "install".into(),
                args: vec!["-m".into(), "0755".into(), "-d".into(), "/etc/apt/keyrings".into()],
                env: vec![],
                as_user: None,
            },
            InstallAction::DownloadScript {
                url: key_url.clone(),
                sha256: String::new(),
                run_as: "root".into(),
                env: vec![],
            },
            InstallAction::Exec {
                cmd: "chmod".into(),
                args: vec!["a+r".into(), "/etc/apt/keyrings/docker.asc".into()],
                env: vec![],
                as_user: None,
            },
            InstallAction::AptRepoAdd {
                name: "docker".into(),
                key_url,
                sources_line,
                sha256: String::new(),
            },
            InstallAction::AptInstall {
                packages: vec![
                    "docker-ce".into(),
                    "docker-ce-cli".into(),
                    "containerd.io".into(),
                    "docker-compose-plugin".into(),
                ],
            },
            InstallAction::Systemctl {
                unit: "docker".into(),
                op: "enable".into(),
            },
            InstallAction::Systemctl {
                unit: "docker".into(),
                op: "start".into(),
            },
            InstallAction::UserCreate {
                name: ctx.target_user.clone(),
                groups: vec!["docker".into()],
                shell: "/bin/bash".into(),
            },
            InstallAction::WriteFile {
                path: "/etc/docker/daemon.json".into(),
                content: r#"{"log-driver":"json-file","log-opts":{"max-size":"10m","max-file":"3"}}"#.into(),
                mode: 0o644,
                backup: true,
            },
            InstallAction::Systemctl {
                unit: "docker".into(),
                op: "restart".into(),
            },
        ])
    }

    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> ModuleResult<ApplyOutcome> {
        let actions = self.plan(ctx).await?;
        crate::executor::command::execute_actions(&actions, &tx, ctx.is_dry_run).await
    }

    async fn verify(&self, _ctx: &Context) -> ModuleResult<VerifyResult> {
        if which::which("docker").is_ok() {
            Ok(VerifyResult::Installed)
        } else {
            Ok(VerifyResult::NotInstalled)
        }
    }
}

fn get_codename() -> String {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|c| {
            c.lines()
                .find(|l| l.starts_with("VERSION_CODENAME="))
                .and_then(|l| l.strip_prefix("VERSION_CODENAME="))
                .map(|s| s.trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "bookworm".into())
}
