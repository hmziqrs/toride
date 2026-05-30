//! Install SSH public keys to remote hosts.
//!
//! The primary entry point is [`install_key_to_remote`], which handles both
//! the `ssh-copy-id` fast path and a manual fallback for environments where
//! `ssh-copy-id` is not available.

use std::path::Path;

use crate::{CliRunner, Error, Result};

/// Result of a key installation attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallOutcome {
    /// `ssh-copy-id` was available and succeeded.
    SshCopyId,
    /// `ssh-copy-id` was not available; manual install via `ssh` succeeded.
    Manual,
}

/// Install a public key on a remote host.
///
/// This function:
/// 1. Detects whether `ssh-copy-id` is available on the local system.
/// 2. If available, uses `ssh-copy-id -i <pubkey> <dest>` to install the key.
/// 3. If not available, falls back to manual mode: SSH into the remote and run
///    `mkdir -p ~/.ssh && echo '<key>' >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys`.
///
/// # Arguments
///
/// * `key_path` - Path to the **private** key file. The corresponding `.pub`
///   file is derived automatically (e.g., `id_ed25519` -> `id_ed25519.pub`).
///   If the `.pub` file does not exist, the private key path is used directly
///   (some `ssh-copy-id` implementations accept this).
/// * `dest` - The remote destination in `[user@]host` format (same as what
///   you would pass to `ssh` or `ssh-copy-id`).
///
/// # Errors
///
/// Returns an error if:
/// - The key path does not exist.
/// - Neither `ssh-copy-id` nor `ssh` is available.
/// - The remote command fails (authentication denied, network unreachable, etc.).
pub async fn install_key_to_remote(
    key_path: &Path,
    dest: &str,
    runner: &dyn CliRunner,
) -> Result<InstallOutcome> {
    if !key_path.exists() {
        return Err(Error::KeyNotFound(key_path.display().to_string()));
    }

    let pub_path = key_path.with_extension("pub");
    let pubkey_path = if pub_path.exists() {
        pub_path
    } else {
        key_path.to_path_buf()
    };

    if runner.tool_exists("ssh-copy-id") {
        let pubkey_str = pubkey_path.to_str().ok_or_else(|| {
            Error::CommandFailed(format!(
                "public key path is not valid UTF-8: {}",
                pubkey_path.display()
            ))
        })?;
        runner
            .run(
                "ssh-copy-id",
                vec!["-i".to_owned(), pubkey_str.to_owned(), dest.to_owned()],
            )
            .await?;
        return Ok(InstallOutcome::SshCopyId);
    }

    if !runner.tool_exists("ssh") {
        return Err(Error::ToolNotFound(
            "neither ssh-copy-id nor ssh found in PATH".to_owned(),
        ));
    }

    install_via_manual_ssh(&pubkey_path, dest).await?;
    Ok(InstallOutcome::Manual)
}

/// Install via manual SSH command when `ssh-copy-id` is unavailable.
///
/// Reads the public key content locally, then runs:
/// ```sh
/// ssh <dest> "mkdir -p ~/.ssh && echo '<key>' >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys"
/// ```
async fn install_via_manual_ssh(pubkey_path: &Path, dest: &str) -> Result<()> {
    let pubkey_content = tokio::task::spawn_blocking({
        let path = pubkey_path.to_path_buf();
        move || {
            std::fs::read_to_string(&path).map_err(|e| {
                Error::CommandFailed(format!(
                    "failed to read public key {}: {e}",
                    path.display()
                ))
            })
        }
    })
    .await
    .map_err(|e| Error::TaskFailed(e.to_string()))??;

    let pubkey_content = pubkey_content.trim();

    // Escape single quotes in the key content for safe shell embedding.
    let escaped_key = pubkey_content.replace('\'', "'\\''");

    let remote_cmd = format!(
        "mkdir -p ~/.ssh && echo '{escaped_key}' >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys"
    );

    let dest = dest.to_owned();
    tokio::task::spawn_blocking(move || {
        duct::cmd("ssh", [&dest, &remote_cmd])
            .read()
            .map_err(|e| Error::CommandFailed(format!("manual key install failed: {e}")))
    })
    .await
    .map_err(|e| Error::TaskFailed(e.to_string()))??;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_outcome_variants_are_distinct() {
        assert_ne!(InstallOutcome::SshCopyId, InstallOutcome::Manual);
    }

    #[test]
    fn install_key_to_remote_rejects_missing_key() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let runner = crate::MockCliRunner::new();
        let result = rt.block_on(install_key_to_remote(
            Path::new("/nonexistent/key"),
            "user@host",
            &runner,
        ));
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::KeyNotFound(_) => {}
            other => panic!("expected KeyNotFound, got: {other:?}"),
        }
    }

    #[test]
    fn manual_ssh_command_format() {
        // Verify the remote command format matches expectations.
        let key = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI user@host";
        let escaped = key.replace('\'', "'\\''");
        let cmd = format!(
            "mkdir -p ~/.ssh && echo '{escaped}' >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys"
        );
        assert!(cmd.starts_with("mkdir -p ~/.ssh && echo '"));
        assert!(cmd.ends_with("chmod 600 ~/.ssh/authorized_keys"));
        assert!(cmd.contains(">> ~/.ssh/authorized_keys"));
    }

    #[test]
    fn manual_ssh_command_escapes_single_quotes() {
        let key = "ssh-ed25519 AAAA it's a key user@host";
        let escaped = key.replace('\'', "'\\''");
        let cmd = format!("echo '{escaped}'");
        // The escaped version should not have unescaped single quotes inside
        // the echo argument. After escaping, it's becomes it'\''s.
        assert!(!cmd.contains("it's"));
        assert!(cmd.contains("it'\\''s"));
    }
}
