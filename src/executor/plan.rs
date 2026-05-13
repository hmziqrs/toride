use crate::tui::model::{ModuleId, Plan, PlanAction, PlanActionStatus};
use crate::modules::registry;

pub async fn generate_plan(module_ids: &[ModuleId]) -> color_eyre::Result<Plan> {
    let reg = registry();
    let ctx = crate::modules::Context {
        is_dry_run: false,
        is_test: std::env::var("TORIDE_E2E").is_ok(),
        target_user: String::new(),
        ssh_public_key: String::new(),
    };

    let mut actions = Vec::new();

    for id in module_ids {
        if let Some(module) = reg.get(id) {
            match module.plan(&ctx).await {
                Ok(module_actions) => {
                    for ma in module_actions {
                        actions.push(PlanAction {
                            module_id: *id,
                            label: ma.to_shell_preview(),
                            status: PlanActionStatus::Pending,
                        });
                    }
                }
                Err(e) => {
                    actions.push(PlanAction {
                        module_id: *id,
                        label: format!("ERROR: {}", e),
                        status: PlanActionStatus::Failed,
                    });
                }
            }
        }
    }

    Ok(Plan {
        actions,
        generated_at: std::time::Instant::now(),
    })
}
