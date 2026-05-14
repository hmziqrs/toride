use async_trait::async_trait;
use crate::modules::*;
use crate::tui::model::{Category, ModuleId};

pub struct DbDump;

#[async_trait]
impl SetupModule for DbDump {
    fn id(&self) -> ModuleId { ModuleId::DbDump }
    fn name(&self) -> &'static str { "Database Dump Helpers" }
    fn description(&self) -> &'static str { "Install pg_dump, mysqldump, and backup helper scripts" }
    fn dependencies(&self) -> Vec<ModuleId> { vec![] }
    fn conflicts(&self) -> Vec<ModuleId> { vec![] }
    fn category(&self) -> Category { Category::Backup }

    async fn preflight(&self, _ctx: &Context) -> ModuleResult<PreflightResult> {
        Ok(PreflightResult::Ok)
    }

    async fn plan(&self, _ctx: &Context) -> ModuleResult<Vec<InstallAction>> {
        let script = r#"#!/bin/bash
set -euo pipefail
BACKUP_DIR="${1:-/var/backups/toride/db}"
mkdir -p "$BACKUP_DIR"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

if command -v pg_dump &>/dev/null; then
    sudo -u postgres pg_dumpall | gzip > "$BACKUP_DIR/postgres_${TIMESTAMP}.sql.gz"
fi

if command -v mysqldump &>/dev/null; then
    mysqldump --all-databases --single-transaction | gzip > "$BACKUP_DIR/mysql_${TIMESTAMP}.sql.gz"
fi

# Keep last 7 days
find "$BACKUP_DIR" -name "*.gz" -mtime +7 -delete
"#;
        Ok(vec![
            InstallAction::AptInstall {
                packages: vec!["postgresql-client".into(), "default-mysql-client".into()],
            },
            InstallAction::Exec {
                cmd: "mkdir".into(),
                args: vec!["-p".into(), "/var/backups/toride/db".into()],
                env: vec![],
                as_user: None,
            },
            InstallAction::WriteFile {
                path: "/usr/local/bin/toride-db-backup".into(),
                content: script.into(),
                mode: 0o755,
                backup: false,
            },
        ])
    }

    async fn apply(&self, ctx: &Context, tx: ProgressTx) -> ModuleResult<ApplyOutcome> {
        let actions = self.plan(ctx).await?;
        crate::executor::command::execute_actions(&actions, &tx, ctx.is_dry_run).await
    }

    async fn verify(&self, _ctx: &Context) -> ModuleResult<VerifyResult> {
        if std::path::Path::new("/usr/local/bin/toride-db-backup").exists() {
            Ok(VerifyResult::Installed)
        } else {
            Ok(VerifyResult::NotInstalled)
        }
    }
}
