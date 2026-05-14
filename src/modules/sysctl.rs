use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct SysctlHardening;

#[async_trait]
impl SetupModule for SysctlHardening {
    fn id(&self) -> ModuleId { ModuleId::SysctlHardening }
    fn name(&self) -> &'static str { "Kernel Hardening" }
    fn description(&self) -> &'static str { "Sysctl security hardening for network, memory, and filesystem" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::FirewallAndSecurity }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        let content = "\
# Network hardening
net.ipv4.ip_forward = 0
net.ipv4.conf.all.send_redirects = 0
net.ipv4.conf.default.send_redirects = 0
net.ipv4.conf.all.accept_redirects = 0
net.ipv4.conf.default.accept_redirects = 0
net.ipv4.conf.all.accept_source_route = 0
net.ipv4.conf.default.accept_source_route = 0
net.ipv4.conf.all.log_martians = 1
net.ipv4.conf.default.log_martians = 1
net.ipv4.icmp_echo_ignore_broadcasts = 1
net.ipv4.icmp_ignore_bogus_error_responses = 1
net.ipv4.tcp_syncookies = 1
net.ipv4.conf.all.rp_filter = 1
net.ipv4.conf.default.rp_filter = 1

# Memory protection
kernel.exec-shield = 1
kernel.randomize_va_space = 2

# Filesystem
fs.suid_dumpable = 0

# Restrict dmesg
kernel.dmesg_restrict = 1

# Restrict kernel pointers
kernel.kptr_restrict = 2
";
        Ok(vec![
            InstallAction::WriteFile {
                path: "/etc/sysctl.d/99-toride.conf".into(),
                content: content.into(),
                mode: 0o644,
                backup: true,
            },
            InstallAction::Exec {
                cmd: "sysctl".into(),
                args: vec!["--system".into()],
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
        if std::path::Path::new("/etc/sysctl.d/99-toride.conf").exists() {
            Ok(VerifyResult::Installed)
        } else {
            Ok(VerifyResult::NotInstalled)
        }
    }
}
