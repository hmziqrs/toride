pub mod command;
pub mod dry_run;
pub mod logs;
pub mod plan;

use crate::tui::model::Outcome;
use crate::modules::registry;

pub async fn execute_plan(
    plan: &crate::tui::model::Plan,
    tx: tokio::sync::mpsc::UnboundedSender<crate::tui::model::ProgressEvent>,
    cancel: tokio_util::sync::CancellationToken,
    ctx: crate::modules::Context,
) -> color_eyre::Result<Outcome> {
    let reg = registry();

    let mut failed = Vec::new();

    // Wait for cloud-init to finish if present
    if std::path::Path::new("/usr/bin/cloud-init").exists() {
        let _ = tx.send(crate::tui::model::ProgressEvent::StepLog {
            action_idx: 0,
            line: "Waiting for cloud-init to complete...".into(),
        });
        let _ = tokio::process::Command::new("cloud-init")
            .args(["status", "--wait"])
            .output()
            .await;
    }

    for (idx, action) in plan.actions.iter().enumerate() {
        if cancel.is_cancelled() {
            return Ok(Outcome::Cancelled);
        }

        let _ = tx.send(crate::tui::model::ProgressEvent::StepStart {
            action_idx: idx,
            label: action.label.clone(),
        });

        if let Some(module) = reg.get(&action.module_id) {
            let module_tx = tx.clone();
            match module.apply(&ctx, module_tx).await {
                Ok(_) => {
                    logs::log_action_jsonl(action.module_id.label(), &action.label, "ok");
                    let _ = tx.send(crate::tui::model::ProgressEvent::StepDone {
                        action_idx: idx,
                        exit_code: 0,
                        duration_ms: 0,
                    });
                }
                Err(e) => {
                    logs::log_action_jsonl(action.module_id.label(), &action.label, &format!("failed: {}", e));
                    let _ = tx.send(crate::tui::model::ProgressEvent::StepFail {
                        action_idx: idx,
                        error: e.to_string(),
                    });
                    failed.push(action.module_id);
                }
            }
        }
    }

    if failed.is_empty() {
        Ok(Outcome::Success)
    } else {
        Ok(Outcome::PartialSuccess { failed })
    }
}
