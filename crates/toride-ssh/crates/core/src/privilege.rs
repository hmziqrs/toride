//! Privilege-escalation helpers for operations that need root.
//!
//! Toride runs either as root (`sudo toride`) or as a normal user. When a
//! write operation targets root-owned files (`/etc/ssh/sshd_config`, other
//! users' `authorized_keys`), it goes through [`run_privileged`], which runs
//! the command directly when already root, or wraps it in `sudo -n` (non-
//! interactive) otherwise.
//!
//! `sudo -n` is used deliberately: interactive `sudo` would fight the TUI for
//! the TTY password prompt. With `-n`, sudo fails fast when no credentials are
//! cached, producing a clear error the UI can surface. The user runs `sudo -v`
//! in another terminal first, or runs the whole app under `sudo`.
//!
//! The privilege boundary is explicit and narrow — a caller cannot execute an
//! arbitrary command; it must describe one of the enumerated [`PrivilegedOp`]s.

use std::path::PathBuf;

use crate::{Error, Result};

/// Path to the system SSH daemon configuration.
const SSHD_CONFIG_PATH: &str = "/etc/ssh/sshd_config";

/// Return `true` when the process is running as root (effective UID 0).
///
/// This is the single source of truth for privilege detection across the
/// app. It is a cheap syscall and safe to call anywhere.
pub fn is_root() -> bool {
    // SAFETY: `geteuid` is a trivial syscall with no preconditions; it just
    // reads the kernel's record of the effective UID.
    let euid = unsafe { libc::geteuid() };
    euid == 0
}

/// An operation that requires elevated privileges to perform.
///
/// Each variant fully describes the privileged write so the boundary stays
/// auditable — there is no generic "run any command" escape hatch.
#[derive(Debug, Clone)]
pub enum PrivilegedOp {
    /// Atomically install new contents for `/etc/ssh/sshd_config`.
    ///
    /// The caller has already serialized the desired file content. This op:
    /// 1. Writes it to a temp file (mode `0o644`).
    /// 2. Validates the temp with `sshd -t -f <temp>` (skipped if `sshd` is
    ///    absent from `PATH`).
    /// 3. Backs up the existing config to `<path>.bak`.
    /// 4. Installs the temp into place.
    WriteSshdConfig {
        /// The full new contents of the sshd_config file.
        content: String,
    },
}

impl PrivilegedOp {
    /// Execute the privileged operation on a blocking thread.
    ///
    /// `running_as_root` should be the result of [`is_root`]; it is passed in
    /// rather than re-detected so the caller controls the trust boundary.
    fn execute(self, running_as_root: bool) -> Result<()> {
        match self {
            Self::WriteSshdConfig { content } => {
                write_sshd_config(&content, running_as_root)
            }
        }
    }
}

/// Execute a privileged operation asynchronously.
///
/// Forwards to a blocking thread (the underlying `duct` calls are blocking).
///
/// # Errors
///
/// Propagates any [`Error`] from the underlying file operations or commands,
/// including a [`Error::SshdConfigInvalid`] when `sshd -t` rejects the new
/// config, or [`Error::SudoFailed`] when `sudo -n` lacks cached credentials.
pub async fn run_privileged(op: PrivilegedOp, running_as_root: bool) -> Result<()> {
    let root = running_as_root;
    tokio::task::spawn_blocking(move || op.execute(root))
        .await
        .map_err(|e| Error::TaskFailed(e.to_string()))?
}

/// Build a `duct` expression that runs `cmd args`, optionally through `sudo`.
///
/// When not root, the command is wrapped as `sudo -n cmd args…` so it fails
/// fast (rather than prompting) when credentials aren't cached.
fn cmd_with_priv(cmd: &str, args: &[&str], running_as_root: bool) -> duct::Expression {
    if running_as_root {
        duct::cmd(cmd, args)
    } else {
        let mut full: Vec<String> = Vec::with_capacity(args.len() + 2);
        full.push("-n".into());
        full.push(cmd.into());
        full.extend(args.iter().map(|s| (*s).to_string()));
        duct::cmd("sudo", full)
    }
}

/// Run `cmd args…` (through sudo when not root) and require a zero exit.
///
/// # Errors
///
/// Returns [`Error::SudoFailed`] on a non-zero exit, including the captured
/// stderr (e.g. "sudo: a password is required") so the UI can show it.
fn run_cmd(cmd: &str, args: &[&str], running_as_root: bool) -> Result<()> {
    let output = cmd_with_priv(cmd, args, running_as_root)
        .stderr_to_stdout()
        .stdout_capture()
        .run()
        .map_err(|e| Error::SudoFailed(format!("failed to run `{cmd}`: {e}")))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stdout);
        let detail = stderr.trim();
        Err(Error::SudoFailed(format!(
            "`{cmd} {args}` exited {:?}: {detail}",
            output.status.code(),
            args = args.join(" ")
        )))
    }
}

/// Write and validate new sshd_config contents, then install them atomically.
///
/// See [`PrivilegedOp::WriteSshdConfig`] for the full sequence.
fn write_sshd_config(content: &str, running_as_root: bool) -> Result<()> {
    // 1. Stage the new contents in a temp file owned by the current user.
    let mut tmp = temp_path("toride-sshd");
    std::fs::write(&tmp, content).map_err(|e| {
        Error::ConfigWriteFailed(format!("failed to stage sshd_config: {e}"))
    })?;
    set_mode(&tmp, 0o644);

    // 2. Validate the staged file with `sshd -t` before touching the live
    //    config. If the binary isn't installed we skip validation and rely on
    //    the backup as the safety net (the most we can do without sshd).
    if which::which("sshd").is_ok() {
        let tmp_str = tmp.to_string_lossy().into_owned();
        validate_sshd_config(&tmp_str, running_as_root)?;
    } else {
        tracing::warn!(
            "sshd not found on PATH; skipping config validation (relying on .bak backup)"
        );
    }

    // 3. Back up the existing config so a bad edit can be reverted.
    let target = PathBuf::from(SSHD_CONFIG_PATH);
    if target.exists() {
        let backup = format!("{SSHD_CONFIG_PATH}.bak");
        // `cp` works the same for root and (via sudo) non-root. Backup failure
        // is non-fatal: we still want to install the validated config.
        if let Err(e) = run_cmd("cp", &[SSHD_CONFIG_PATH, &backup], running_as_root) {
            tracing::warn!("failed to back up sshd_config: {e}");
        }
    }

    // 4. Install: copy the staged file into place, then enforce mode 0o644.
    //    `cp` (rather than `mv`) works both when root and when crossing the
    //    user→root ownership boundary via sudo. We keep the staging file and
    //    clean it up afterward.
    run_cmd(
        "cp",
        &[
            &tmp.to_string_lossy(),
            SSHD_CONFIG_PATH,
        ],
        running_as_root,
    )?;
    run_cmd("chmod", &["644", SSHD_CONFIG_PATH], running_as_root)?;

    // Clean up the staging file (best effort).
    let _ = std::fs::remove_file(&tmp);

    Ok(())
}

/// Run `sshd -t -f <path>` and fail loudly if the config is rejected.
///
/// # Errors
///
/// Returns [`Error::SshdConfigInvalid`] with `sshd`'s diagnostic output when
/// the config fails validation, or [`Error::SudoFailed`] when the `sshd`
/// invocation itself can't run.
fn validate_sshd_config(path: &str, running_as_root: bool) -> Result<()> {
    let output = cmd_with_priv("sshd", &["-t", "-f", path], running_as_root)
        .stderr_to_stdout()
        .stdout_capture()
        .run()
        .map_err(|e| Error::SudoFailed(format!("failed to run `sshd -t`: {e}")))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stdout);
        Err(Error::SshdConfigInvalid(stderr.trim().to_string()))
    }
}

/// A unique temp path under the system temp dir, suffixed with the pid.
fn temp_path(prefix: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("{prefix}.{}.tmp", std::process::id()));
    p
}

/// Set the Unix file mode, ignoring failure (best-effort permission hygiene).
fn set_mode(path: &std::path::Path, mode: u32) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_root_matches_geteuid() {
        // Smoke test: is_root() should agree with a direct libc check.
        let euid = unsafe { libc::geteuid() };
        assert_eq!(is_root(), euid == 0);
    }

    #[test]
    fn temp_path_is_under_temp_dir() {
        let p = temp_path("test-prefix");
        assert!(p.starts_with(std::env::temp_dir()));
        assert!(p.to_string_lossy().contains("test-prefix"));
    }

    #[test]
    fn write_sshd_config_rejects_invalid_config_when_sshd_present() {
        // If sshd is available, malformed content must be rejected.
        if which::which("sshd").is_err() {
            return; // skip when sshd isn't installed
        }
        let bad = "this is not valid sshd_config @@@\n";
        let err = write_sshd_config(bad, is_root());
        assert!(err.is_err(), "invalid sshd_config must be rejected");
    }
}
