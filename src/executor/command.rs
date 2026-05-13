use crate::modules::{ApplyOutcome, InstallAction, ModuleResult};
use crate::tui::model::ProgressEvent;

pub type ProgressTx = tokio::sync::mpsc::UnboundedSender<ProgressEvent>;

pub async fn execute_actions(
    actions: &[InstallAction],
    tx: &tokio::sync::mpsc::UnboundedSender<ProgressEvent>,
    dry_run: bool,
) -> ModuleResult<ApplyOutcome> {
    if dry_run {
        return Ok(ApplyOutcome::Skipped);
    }

    for (idx, action) in actions.iter().enumerate() {
        let preview = action.to_shell_preview();
        let _ = tx.send(ProgressEvent::StepLog {
            action_idx: idx,
            line: preview,
        });

        let result = execute_single(action).await;
        match result {
            Ok(output) => {
                for line in output.lines() {
                    let _ = tx.send(ProgressEvent::StepLog {
                        action_idx: idx,
                        line: line.to_string(),
                    });
                }
                let _ = tx.send(ProgressEvent::StepDone {
                    action_idx: idx,
                    exit_code: 0,
                    duration_ms: 0,
                });
            }
            Err(e) => {
                let _ = tx.send(ProgressEvent::StepFail {
                    action_idx: idx,
                    error: e.to_string(),
                });
                return Err(e);
            }
        }
    }

    Ok(ApplyOutcome::Changed)
}

async fn execute_single(action: &InstallAction) -> ModuleResult<String> {
    match action {
        InstallAction::AptInstall { .. } => {
            run_cmd("apt-get", &["install", "-y"]).await
        }
        InstallAction::Exec { cmd, args, .. } => {
            let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            run_cmd(cmd, &args).await
        }
        InstallAction::Systemctl { unit, op } => {
            run_cmd("systemctl", &[op.as_str(), unit.as_str()]).await
        }
        InstallAction::UfwRule { rule } => {
            run_cmd("ufw", &[rule.as_str()]).await
        }
        _ => Ok(String::new()),
    }
}

async fn run_cmd(cmd: &str, args: &[&str]) -> ModuleResult<String> {
    use tokio::io::AsyncBufReadExt;
    use tokio_stream::wrappers::LinesStream;
    use tokio_stream::StreamExt;

    let mut child = tokio::process::Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| crate::modules::ModuleError::Exec(e.to_string()))?;

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    let stdout_lines = LinesStream::new(tokio::io::BufReader::new(stdout).lines());
    let stderr_lines = LinesStream::new(tokio::io::BufReader::new(stderr).lines());

    let mut merged = stdout_lines.merge(stderr_lines);
    let mut output = String::new();

    while let Some(Ok(line)) = merged.next().await {
        output.push_str(&line);
        output.push('\n');
    }

    let status = child.wait().await
        .map_err(|e| crate::modules::ModuleError::Exec(e.to_string()))?;

    if status.success() {
        Ok(output)
    } else {
        Err(crate::modules::ModuleError::Exec(format!("{} exited with {}", cmd, status)))
    }
}
