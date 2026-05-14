use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct CloudflareHttp;

#[async_trait]
impl SetupModule for CloudflareHttp {
    fn id(&self) -> ModuleId { ModuleId::CloudflareHttp }
    fn name(&self) -> &'static str { "Cloudflare-only HTTP/S" }
    fn description(&self) -> &'static str { "Restrict ports 80/443 to Cloudflare IP ranges only" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![ModuleId::Ufw] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::FirewallAndSecurity }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        let mut actions = vec![
            InstallAction::Exec {
                cmd: "mkdir".into(),
                args: vec!["-p".into(), "/var/lib/toride".into()],
                env: vec![],
                as_user: None,
            },
        ];

        let ipv4_ranges = [
            "173.245.48.0/20", "103.21.244.0/22", "103.22.200.0/22",
            "103.31.4.0/22", "141.101.64.0/18", "108.162.192.0/18",
            "190.93.240.0/20", "188.114.96.0/20", "197.234.240.0/22",
            "198.41.128.0/17", "162.158.0.0/15", "104.16.0.0/13",
            "104.24.0.0/14", "172.64.0.0/13", "131.0.72.0/22",
        ];
        let ipv6_ranges = [
            "2400:cb00::/32", "2606:4700::/32", "2803:f800::/32",
            "2405:b500::/32", "2405:8100::/32", "2a06:98c0::/29",
            "2c0f:f248::/32",
        ];

        for range in ipv4_ranges.iter().chain(ipv6_ranges.iter()) {
            actions.push(InstallAction::UfwRule {
                rule: format!("allow from {} to any port 80", range),
            });
            actions.push(InstallAction::UfwRule {
                rule: format!("allow from {} to any port 443", range),
            });
        }

        actions.push(InstallAction::Exec {
            cmd: "bash".into(),
            args: vec!["-c".into(), "ufw deny 80 && ufw deny 443".into()],
            env: vec![],
            as_user: None,
        });

        actions.push(InstallAction::WriteFile {
            path: "/var/lib/toride/cloudflare-ips.txt".into(),
            content: format!("{}\n{}\n", ipv4_ranges.join("\n"), ipv6_ranges.join("\n")),
            mode: 0o644,
            backup: false,
        });

        Ok(actions)
    }

    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> ModuleResult<ApplyOutcome> {
        let actions = self.plan(ctx).await?;
        crate::executor::command::execute_actions(&actions, &tx, ctx.is_dry_run).await
    }

    async fn verify(&self, _ctx: &Context) -> ModuleResult<VerifyResult> {
        if std::path::Path::new("/var/lib/toride/cloudflare-ips.txt").exists() {
            Ok(VerifyResult::Installed)
        } else {
            Ok(VerifyResult::NotInstalled)
        }
    }
}
