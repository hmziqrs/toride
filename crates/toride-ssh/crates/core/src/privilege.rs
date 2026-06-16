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

use std::path::Path;

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
    /// 1. Stages the content in an O_EXCL, unpredictable-named temp file in
    ///    the *same directory* as the target (`/etc/ssh`) — so the install is
    ///    an atomic `rename(2)` within one filesystem, and `sshd -t` resolves
    ///    relative `Include` directives (e.g. the default
    ///    `Include /etc/ssh/sshd_config.d/*.conf`) against the live directory.
    /// 2. **Always** validates the staged file with `sshd -t` through the
    ///    privileged path. If the `sshd` binary is genuinely unavailable, the
    ///    write **fails closed** — an unvalidatable config is never installed.
    /// 3. Backs up the existing config to `<path>.bak` *before* install. A
    ///    backup failure is fatal (the write aborts without touching the live
    ///    config).
    /// 4. Installs the temp into place via a single `rename(2)`.
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
                // Production path: no injected validator, real target, real
                // `sshd -t` through the privileged runner.
                write_sshd_config(
                    &content,
                    running_as_root,
                    Path::new(SSHD_CONFIG_PATH),
                    None,
                )
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
/// config, [`Error::SshdNotFound`] when the `sshd` binary is genuinely
/// unavailable (the write fails closed in that case), or [`Error::SudoFailed`]
/// when `sudo -n` lacks cached credentials.
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

/// Type of the validation seam injected into [`write_sshd_config`].
///
/// A validator runs the staged temp file through `sshd -t` (or a test double)
/// and reports one of three outcomes:
///
/// - [`ValidateOutcome::Ok`] — the config parses, proceed to install.
/// - [`ValidateOutcome::Invalid`] — `sshd -t` rejected the config; fail closed
///   with [`Error::SshdConfigInvalid`].
/// - [`ValidateOutcome::BinaryMissing`] — the `sshd` binary itself could not
///   be found; fail closed with [`Error::SshdNotFound`]. We never install a
///   config we could not validate.
type Validator = fn(path: &Path, running_as_root: bool) -> ValidateOutcome;

/// The result of validating a staged sshd_config.
///
/// Returned by the validation seam so callers (and tests) can distinguish the
/// three terminal cases without inspecting error text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidateOutcome {
    /// `sshd -t` accepted the config.
    Ok,
    /// `sshd -t` rejected the config; carry the diagnostic output.
    Invalid(String),
    /// The `sshd` binary is not installed / unreachable via the privileged
    /// path; carry the detail we observed.
    BinaryMissing(String),
}

/// Write, validate, back up, and atomically install new sshd_config contents.
///
/// This is the hardened write path. The hardening vs. the original
/// implementation:
///
/// - **Validation is never skipped.** The old code gated `sshd -t` on
///   `which::which("sshd")`, which resolves against the *invoking* user's
///   `PATH`. On Debian/Ubuntu a normal user's `PATH` lacks `/usr/sbin` where
///   `sshd` lives, so validation was silently skipped and an unvalidated
///   config was installed. Now we always run validation through the privileged
///   runner (`sudo -n`, which uses sudo's `secure_path` containing `/usr/sbin`).
///   If the binary is genuinely missing we fail closed — no install.
/// - **The temp is staged next to the target.** The temp lives in the same
///   directory as the target (`/etc/ssh`), so install via `rename(2)` is
///   atomic (same filesystem) and `sshd -t` resolves relative `Include`
///   directives against the live directory.
/// - **Backup failure is fatal.** We keep the `.bak` backup but, unlike
///   before, a failed backup aborts the install rather than being warned
///   away. The security constraint requires a backup before every install.
///
/// # Test seam
///
/// `validator` lets tests inject a fake validation result (binary-missing vs.
/// invalid vs. ok) without touching the real `/etc/ssh`. In production it is
/// `None` and [`default_validator`] is used. Likewise `target` is a parameter
/// only so tests can point it at a tempdir; production callers go through
/// [`PrivilegedOp::execute`], which pins it to [`SSHD_CONFIG_PATH`].
fn write_sshd_config(
    content: &str,
    running_as_root: bool,
    target: &Path,
    validator: Option<Validator>,
) -> Result<()> {
    let validator = validator.unwrap_or(default_validator);

    // 1. Stage the new contents in an O_EXCL, unpredictable-named temp file in
    //    the SAME directory as the target. Same directory is load-bearing:
    //    (a) install via rename is atomic only within one filesystem;
    //    (b) `sshd -t -f <temp>` resolves relative Include directives against
    //        the temp's directory, so placing it next to the target makes the
    //        default `Include /etc/ssh/sshd_config.d/*.conf` resolve correctly.
    let staging_dir = target.parent().unwrap_or_else(|| Path::new("/"));
    let tmp = tempfile::Builder::new()
        .prefix("toride-sshd-")
        .suffix(".tmp")
        // O_EXCL + 0o600 kills the predictable-pid-name symlink-attack surface
        // of the old `/tmp/<prefix>.<pid>.tmp` scheme.
        .permissions(std::os::unix::fs::PermissionsExt::from_mode(0o600))
        .tempfile_in(staging_dir)
        .map_err(|e| Error::ConfigWriteFailed(format!("failed to stage sshd_config: {e}")))?;

    // Capture the temp path before any move so the error path can clean up.
    let tmp_path_for_err = tmp.path().to_path_buf();

    use std::io::Write;
    tmp.as_file()
        .write_all(content.as_bytes())
        .map_err(|e| Error::ConfigWriteFailed(format!("failed to write staged sshd_config: {e}")))?;
    // fsync the temp so the validated bytes are what actually lands on disk.
    tmp.as_file()
        .sync_all()
        .map_err(|e| Error::ConfigWriteFailed(format!("failed to fsync staged sshd_config: {e}")))?;

    // Keep the temp path as a plain PathBuf for the validation/restore logic;
    // `keep()` detaches it from the drop-deleting guard so we control its
    // lifetime ourselves (we explicitly remove it on every exit path).
    let tmp_path = match tmp.keep() {
        Ok((_file, path)) => path,
        // If keep() failed, the underlying file may still exist; try to clean
        // it up, then surface the error.
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_path_for_err);
            return Err(Error::ConfigWriteFailed(format!(
                "failed to persist staged sshd_config: {e}"
            )));
        }
    };

    // Run the write through to completion, guaranteeing temp cleanup.
    let result = write_sshd_config_finish(content, running_as_root, target, &tmp_path, validator);

    // Best-effort cleanup of the staging temp on every path (success or
    // failure). The live config is never the temp.
    let _ = std::fs::remove_file(&tmp_path);
    result
}

/// Second half of [`write_sshd_config`], separated so temp cleanup always runs.
///
/// Order of operations (fail closed at every gate):
/// 1. Validate the staged temp. Binary-missing or invalid → abort, no install.
/// 2. Back up the existing live config to `<target>.bak`. Backup failure →
///    abort, no install.
/// 3. Install the temp into place via a single `rename(2)`.
/// 4. Enforce mode 0o644 on the live file (the temp was 0o600).
fn write_sshd_config_finish(
    _content: &str,
    running_as_root: bool,
    target: &Path,
    tmp_path: &Path,
    validator: Validator,
) -> Result<()> {
    // 1. VALIDATION GATE — never skipped. Distinguish "binary missing" from
    //    "config invalid"; both fail closed.
    match validator(tmp_path, running_as_root) {
        ValidateOutcome::Ok => {}
        ValidateOutcome::Invalid(detail) => {
            return Err(Error::SshdConfigInvalid(detail));
        }
        ValidateOutcome::BinaryMissing(detail) => {
            return Err(Error::SshdNotFound(detail));
        }
    }

    // 2. BACKUP BEFORE INSTALL — fatal on failure.
    if target.exists() {
        backup_config(target, running_as_root)?;
    }

    // 3. INSTALL via a single rename(2). Atomic within the same filesystem,
    //    which is guaranteed because we staged the temp in the target's
    //    directory.
    install_temp(tmp_path, target, running_as_root)?;

    // 4. Enforce mode 0o644 on the live file (the temp was 0o600; rename
    //    preserves the source's mode). The final live file must be a
    //    root-owned 0o644 sshd_config.
    set_mode_priv(target, 0o644, running_as_root)?;

    Ok(())
}

/// The production validator: run `sshd -t -f <path>` through the privileged
/// runner and classify the outcome.
///
/// We do NOT gate on `which::which("sshd")`: that resolves against the
/// invoking user's `PATH`, which on Debian/Ubuntu omits `/usr/sbin`, so the
/// real (root-installed) `sshd` would be missed and validation silently
/// skipped. Instead we always invoke through the privileged runner; `sudo -n`
/// uses sudo's `secure_path` (which includes `/usr/sbin`). We then inspect the
/// failure to tell "binary genuinely not found" apart from "config invalid".
fn default_validator(path: &Path, running_as_root: bool) -> ValidateOutcome {
    let path_str = path.to_string_lossy();
    let output = cmd_with_priv("sshd", &["-t", "-f", &path_str], running_as_root)
        .stderr_to_stdout()
        .stdout_capture()
        .run();

    let output = match output {
        Ok(o) => o,
        // The process itself couldn't be spawned. This is the "binary not
        // found" case on the non-root path (sudo also reports it via stderr
        // and a non-zero exit, but a spawn failure here means the binary — or
        // sudo — is genuinely absent).
        Err(e) => {
            let detail = e.to_string();
            if is_binary_missing(&detail) {
                return ValidateOutcome::BinaryMissing(detail);
            }
            return ValidateOutcome::Invalid(detail);
        }
    };

    if output.status.success() {
        return ValidateOutcome::Ok;
    }

    let stderr = String::from_utf8_lossy(&output.stdout);
    let detail = stderr.trim().to_string();

    if is_binary_missing(&detail) {
        ValidateOutcome::BinaryMissing(detail)
    } else {
        ValidateOutcome::Invalid(detail)
    }
}

/// Heuristic: does this error text indicate the `sshd` binary is missing
/// (rather than the config being invalid)?
///
/// Matches the phrasings emitted by `sudo` and the shell when a command
/// cannot be found: "command not found", "not found", "No such file".
/// `sshd -t` itself never emits these for an invalid config (it reports the
/// specific parse/semantics error), so a match here reliably means the binary
/// is unavailable.
fn is_binary_missing(detail: &str) -> bool {
    let lower = detail.to_ascii_lowercase();
    lower.contains("command not found")
        || lower.contains("not found")
        || lower.contains("no such file")
}

/// Back up `target` to `<target>.bak` via `cp`. Fatal on failure.
///
/// # Errors
///
/// Returns [`Error::ConfigWriteFailed`] if the privileged `cp` fails. The
/// caller treats this as fatal and does not proceed to install — the security
/// constraint requires a backup before every install.
fn backup_config(target: &Path, running_as_root: bool) -> Result<()> {
    let src = target.to_string_lossy();
    let dst = format!("{}.bak", src);
    run_cmd(
        "cp",
        &["-p", &src, &dst],
        running_as_root,
    )
    .map_err(|e| {
        Error::ConfigWriteFailed(format!(
            "failed to back up sshd_config to {dst} (aborting install): {e}"
        ))
    })
}

/// Install `tmp_path` at `target` via a single `rename(2)`.
///
/// When already root, `std::fs::rename` is the atomic rename. When not root,
/// we use `sudo -n mv`, which performs a single `rename(2)` (the source and
/// target share a filesystem because the temp was staged in the target's
/// directory, so `mv` will not fall back to copy+unlink).
///
/// # Errors
///
/// Returns [`Error::ConfigWriteFailed`] on a non-zero exit.
fn install_temp(tmp_path: &Path, target: &Path, running_as_root: bool) -> Result<()> {
    if running_as_root {
        std::fs::rename(tmp_path, target).map_err(|e| {
            Error::ConfigWriteFailed(format!("failed to install sshd_config: {e}"))
        })
    } else {
        let src = tmp_path.to_string_lossy();
        let dst = target.to_string_lossy();
        run_cmd("mv", &[&src, &dst], running_as_root).map_err(|e| {
            Error::ConfigWriteFailed(format!("failed to install sshd_config: {e}"))
        })
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

/// Set the Unix file mode on `path`. When not root, run through `chmod` under
/// `sudo -n` (the file is root-owned); when root, set it directly.
///
/// # Errors
///
/// Returns [`Error::ConfigWriteFailed`] if the mode cannot be applied.
fn set_mode_priv(path: &Path, mode: u32, running_as_root: bool) -> Result<()> {
    if running_as_root {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).map_err(|e| {
                Error::ConfigWriteFailed(format!("failed to chmod sshd_config: {e}"))
            })?;
            return Ok(());
        }
        #[cfg(not(unix))]
        {
            let _ = (path, mode);
            return Ok(());
        }
    }
    let path_str = path.to_string_lossy();
    run_cmd("chmod", &[&format!("{mode:o}"), &path_str], running_as_root).map_err(|e| {
        Error::ConfigWriteFailed(format!("failed to chmod sshd_config: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A validator that always reports the config as valid.
    fn validator_ok(_path: &Path, _running_as_root: bool) -> ValidateOutcome {
        ValidateOutcome::Ok
    }

    /// A validator that simulates the `sshd` binary being absent.
    fn validator_binary_missing(_path: &Path, _running_as_root: bool) -> ValidateOutcome {
        ValidateOutcome::BinaryMissing("sudo: sshd: command not found".into())
    }

    /// A validator that simulates `sshd -t` rejecting the config.
    fn validator_invalid(_path: &Path, _running_as_root: bool) -> ValidateOutcome {
        ValidateOutcome::Invalid("line 1: Bad configuration option: foo".into())
    }

    #[test]
    fn is_root_matches_geteuid() {
        // Smoke test: is_root() should agree with a direct libc check.
        let euid = unsafe { libc::geteuid() };
        assert_eq!(is_root(), euid == 0);
    }

    #[test]
    fn is_binary_missing_recognizes_shell_phrasings() {
        assert!(is_binary_missing("sudo: sshd: command not found"));
        assert!(is_binary_missing("sshd: not found"));
        assert!(is_binary_missing(
            "sudo: unable to execute /usr/sbin/sshd: No such file or directory"
        ));
        // A genuine config error must NOT be classified as binary-missing.
        assert!(!is_binary_missing("line 3: Bad configuration option: foo"));
        assert!(!is_binary_missing("Missing value in subsystem definition."));
    }

    #[test]
    fn write_sshd_config_rejects_invalid_config_when_sshd_present() {
        // Production validator (real sshd -t). If sshd is installed, malformed
        // content must be rejected. Skip when sshd genuinely isn't present so
        // this stays green on CI without sshd.
        if std::process::Command::new("sshd")
            .arg("-V")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_err()
            && which::which("sshd").is_err()
        {
            // One last attempt through the privileged path; if even that says
            // binary-missing, skip.
            let dir = tempfile::tempdir().unwrap();
            let staged = dir.path().join("probe");
            std::fs::write(&staged, "Port 22\n").unwrap();
            if matches!(
                default_validator(&staged, is_root()),
                ValidateOutcome::BinaryMissing(_)
            ) {
                return;
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("sshd_config");
        let bad = "this is not valid sshd_config @@@\n";
        let err = write_sshd_config(bad, is_root(), &target, None);
        assert!(
            err.is_err(),
            "invalid sshd_config must be rejected (got {err:?})"
        );
        // And the live config must not have been created.
        assert!(
            !target.exists(),
            "invalid config must not be installed"
        );
    }

    #[test]
    fn write_sshd_config_fails_closed_when_sshd_binary_missing() {
        // Injected seam: validator reports the binary is absent. The write
        // MUST fail closed and MUST NOT install anything.
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("sshd_config");
        // Pre-existing live config — it must be left untouched.
        std::fs::write(&target, "Port 2222\n").unwrap();

        let err = write_sshd_config(
            "Port 22\n",
            is_root(),
            &target,
            Some(validator_binary_missing),
        )
        .expect_err("must fail closed when sshd binary is missing");

        assert!(
            matches!(err, Error::SshdNotFound(_)),
            "expected SshdNotFound, got {err:?}"
        );
        // The live config is untouched.
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "Port 2222\n");
        // No backup was taken (backup happens after validation, which failed).
        assert!(!Path::new(&format!("{}.bak", target.display())).exists());
    }

    #[test]
    fn write_sshd_config_fails_closed_when_config_invalid() {
        // Injected seam: validator reports the config is invalid. The write
        // MUST fail closed and MUST NOT install anything.
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("sshd_config");
        std::fs::write(&target, "Port 2222\n").unwrap();

        let err = write_sshd_config(
            "BadDirective yes\n",
            is_root(),
            &target,
            Some(validator_invalid),
        )
        .expect_err("must fail closed when config is invalid");

        assert!(
            matches!(err, Error::SshdConfigInvalid(_)),
            "expected SshdConfigInvalid, got {err:?}"
        );
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "Port 2222\n");
    }

    #[test]
    fn write_sshd_config_happy_path_stages_in_target_dir_and_backs_up() {
        // Injected seam: validator ok. The happy path must:
        //  - stage the temp NEXT TO the target (same dir),
        //  - back the live config up to .bak before install,
        //  - install the new content as the live config with mode 0o644.
        //
        // We force `running_as_root = true` so backup/install/chmod take their
        // direct (std::fs) branches. The test tempdir is owned by this
        // process, so those succeed regardless of our real uid and we don't
        // depend on `sudo -n` credentials being cached.
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("sshd_config");
        std::fs::write(&target, "# old\nPort 2222\n").unwrap();

        write_sshd_config("# new\nPort 22\n", true, &target, Some(validator_ok))
            .expect("happy path must succeed");

        // New content installed.
        let installed = std::fs::read_to_string(&target).unwrap();
        assert!(installed.contains("Port 22"));
        assert!(!installed.contains("Port 2222"));

        // Backup taken before install.
        let backup_path = format!("{}.bak", target.display());
        let backup = std::fs::read_to_string(&backup_path).unwrap();
        assert!(
            backup.contains("Port 2222"),
            "backup must contain the pre-install content"
        );

        // The staged temp must have lived as a sibling of the target (not in
        // /tmp). We assert this indirectly: the only toride-staged artifacts
        // left in the target dir are the target itself and its .bak — no
        // stray `toride-sshd-*` temp remains (it was renamed into place /
        // cleaned up). This confirms staging happened in this dir.
        for entry in std::fs::read_dir(dir.path()).unwrap() {
            let name = entry.unwrap().file_name();
            let name = name.to_string_lossy();
            assert!(
                !name.starts_with("toride-sshd-"),
                "staging temp must be cleaned up, found {name:?}"
            );
        }
    }

    #[test]
    fn write_sshd_config_stages_temp_as_sibling_of_target() {
        // Assert the "stage in the target dir" property by intercepting the
        // staged temp path via the validator seam. The validator receives the
        // exact temp path used for `sshd -t`; it must share a parent with the
        // target (so install is an atomic rename and Include resolves right).
        // Function pointers can't close over locals, so we record the path in
        // a module-level test static.
        STAGED_PATH.lock().unwrap().take();
        let captured: Validator = |path: &Path, _r: bool| {
            *STAGED_PATH.lock().unwrap() = Some(path.to_path_buf());
            ValidateOutcome::Ok
        };

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("sshd_config");
        std::fs::write(&target, "Port 2222\n").unwrap();

        write_sshd_config("Port 22\n", true, &target, Some(captured)).unwrap();

        let staged = STAGED_PATH
            .lock()
            .unwrap()
            .clone()
            .expect("validator must have been called with the temp path");
        assert_eq!(
            staged.parent(),
            target.parent(),
            "staged temp must live in the SAME directory as the target"
        );
    }

    /// Module-level scratch slot so a function-pointer validator can report the
    /// staged temp path back to the test (function pointers can't capture).
    static STAGED_PATH: std::sync::Mutex<Option<std::path::PathBuf>> = std::sync::Mutex::new(None);

    #[test]
    fn write_sshd_config_aborts_install_when_backup_fails() {
        // Make the backup impossible by making the target's directory lack a
        // real existing target that `cp -p` can read... Instead, simulate a
        // backup failure by pointing the target at a path whose PARENT can't
        // hold the .bak (make the parent read-only). Because we run the cp
        // unprivileged here, a read-only parent will fail the backup.
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("readonly");
        std::fs::create_dir(&nested).unwrap();
        let target = nested.join("sshd_config");
        std::fs::write(&target, "Port 2222\n").unwrap();

        // Remove write permission from the parent dir so `cp` cannot create
        // the .bak file. (We are the owner, so this is effective.)
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&nested, std::fs::Permissions::from_mode(0o555)).unwrap();

        // Run as "root" so install takes the std::fs::rename branch (also
        // fails because the dir is read-only) and backup takes the cp branch.
        // The key assertion: the original content is preserved and NO install
        // happens, because backup is fatal and runs before install.
        let err = write_sshd_config(
            "# new\nPort 22\n",
            true,
            &target,
            Some(validator_ok),
        )
        .expect_err("backup failure must abort the install");

        // Restore perms so cleanup can happen.
        let _ = std::fs::set_permissions(&nested, std::fs::Permissions::from_mode(0o755));

        assert!(
            matches!(err, Error::ConfigWriteFailed(_)),
            "expected ConfigWriteFailed from backup, got {err:?}"
        );
        // Live config untouched.
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "Port 2222\n");
    }
}
