mod generate;
mod inventory;
mod repair;

use std::ffi::OsStr;

use crate::paths::SshPaths;
use crate::{Error, KeyCreateParams, KeyDeleteParams, KeyFormat, Result, SshKey};

/// Get Unix file permissions from metadata.
#[cfg(unix)]
pub(crate) fn get_permissions(path: &std::path::Path) -> Option<crate::Permissions> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = std::fs::metadata(path).ok()?;
    let mode = metadata.permissions().mode();
    // Only keep the lower 12 bits (rwx + setuid/setgid/sticky)
    Some(crate::Permissions {
        mode: mode & 0o7777,
    })
}

#[cfg(not(unix))]
pub(crate) fn get_permissions(_path: &std::path::Path) -> Option<crate::Permissions> {
    None
}

/// Validate a key name to prevent path traversal attacks.
///
/// Key names must not contain path separators, `..` components, or null bytes.
/// Maximum length is 255 bytes (typical filesystem limit).
fn validate_key_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::KeyGenerationFailed("key name must not be empty".to_owned()));
    }
    if name.len() > 255 {
        return Err(Error::KeyGenerationFailed(
            "key name must not exceed 255 bytes".to_owned(),
        ));
    }
    if name.contains('\0') {
        return Err(Error::KeyGenerationFailed(
            "key name must not contain null bytes".to_owned(),
        ));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(Error::KeyGenerationFailed(
            "key name must not contain path separators".to_owned(),
        ));
    }
    if name.contains("..") {
        return Err(Error::KeyGenerationFailed(
            "key name must not contain '..'".to_owned(),
        ));
    }
    Ok(())
}

/// Return a unique backup path by appending a Unix timestamp suffix if the
/// path already exists.  For example, if `foo.bak` exists, returns
/// `foo.bak.1717020000`.
fn unique_backup_path(base: &std::path::Path) -> std::path::PathBuf {
    if !base.exists() {
        return base.to_path_buf();
    }
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let ext = match base.extension() {
        Some(e) => format!("{}.{}", e.to_string_lossy(), ts),
        None => ts.to_string(),
    };
    base.with_extension(ext)
}

/// Key management operations.
pub struct KeyService<'a> {
    paths: &'a SshPaths,
}

impl<'a> KeyService<'a> {
    pub(crate) fn new(paths: &'a SshPaths) -> Self {
        Self { paths }
    }

    /// List all SSH keys found on disk and in the agent.
    pub async fn list(&self) -> Result<Vec<SshKey>> {
        inventory::scan_keys(self.paths).await
    }

    /// Generate a new SSH key pair.
    pub async fn create(&self, params: KeyCreateParams) -> Result<SshKey> {
        validate_key_name(&params.name)?;
        generate::generate_key(self.paths, params).await
    }

    /// Delete a key and optionally its public pair, certificate, agent entry, and config refs.
    pub async fn delete(&self, params: KeyDeleteParams) -> Result<()> {
        validate_key_name(&params.name)?;
        let private_path = self.paths.ssh_dir().join(&params.name);

        if !private_path.exists() {
            return Err(Error::KeyNotFound(params.name.clone()));
        }

        let public_path = private_path.with_extension("pub");

        let cert_path = self.paths.ssh_dir().join(format!("{}-cert.pub", params.name));

        // Destructure to avoid cloning the entire params struct into spawn_blocking.
        let backup = params.backup;
        let remove_public = params.remove_public;
        let remove_certificate = params.remove_certificate;

        tokio::task::spawn_blocking(move || {
            // Backup if requested
            if backup {
                let backup_path = unique_backup_path(&private_path.with_extension("bak"));
                std::fs::rename(&private_path, &backup_path)?;

                if remove_public && public_path.exists() {
                    let stem = public_path.file_stem().unwrap_or(OsStr::new("")).to_string_lossy();
                    let pub_backup_base = public_path.with_file_name(format!("{stem}.pub.bak"));
                    let pub_backup = unique_backup_path(&pub_backup_base);
                    if let Err(e) = std::fs::rename(&public_path, &pub_backup) {
                        tracing::warn!("failed to backup {}: {e}", public_path.display());
                    }
                }

                if remove_certificate && cert_path.exists() {
                    let name = cert_path.file_name().unwrap_or(OsStr::new("")).to_string_lossy();
                    let cert_backup_base = cert_path.with_file_name(format!("{name}.bak"));
                    let cert_backup = unique_backup_path(&cert_backup_base);
                    if let Err(e) = std::fs::rename(&cert_path, &cert_backup) {
                        tracing::warn!("failed to backup {}: {e}", cert_path.display());
                    }
                }
            } else {
                // Remove the private key file
                std::fs::remove_file(&private_path)?;

                // Remove public key companion
                if remove_public && public_path.exists() {
                    std::fs::remove_file(&public_path)?;
                }

                // Remove certificate companion
                if remove_certificate && cert_path.exists() {
                    std::fs::remove_file(&cert_path)?;
                }
            }

            Ok::<(), Error>(())
        })
        .await
        .map_err(|e| Error::TaskFailed(format!("delete task failed: {e}")))??;

        // Remove from agent if requested (non-fatal)
        if params.remove_from_agent {
            remove_key_from_agent(&self.paths.ssh_dir().join(&params.name)).await;
        }

        // Remove from config if requested
        if params.remove_from_config {
            remove_from_config(self.paths, &params.name).await?;
        }

        Ok(())
    }

    /// Derive the `.pub` file from a private key.
    pub async fn repair_public(&self, private_key_path: &std::path::Path) -> Result<()> {
        repair::repair_public_key(private_key_path).await
    }

    /// Rename a key pair (private, public, certificate).
    ///
    /// Renames `~/.ssh/<old_name>` to `~/.ssh/<new_name>` and all companion
    /// files (`.pub`, `-cert.pub`). Does NOT update config references — call
    /// `remove_from_config` for the old name and add new `IdentityFile` entries
    /// separately.
    ///
    /// If the private key rename succeeds but the public key or certificate
    /// rename fails, the operation continues with a warning. This may leave
    /// the key pair in an inconsistent state where the private key has the
    /// new name but the public key retains the old name.
    pub async fn rename(&self, old_name: &str, new_name: &str) -> Result<()> {
        validate_key_name(old_name)?;
        validate_key_name(new_name)?;

        let old_private = self.paths.ssh_dir().join(old_name);
        let new_private = self.paths.ssh_dir().join(new_name);

        if !old_private.exists() {
            return Err(Error::KeyNotFound(old_name.to_owned()));
        }
        if new_private.exists() {
            return Err(Error::KeyExists(new_name.to_owned()));
        }

        let old_public = old_private.with_extension("pub");
        let new_public = new_private.with_extension("pub");
        let old_cert = self.paths.ssh_dir().join(format!("{old_name}-cert.pub"));
        let new_cert = self.paths.ssh_dir().join(format!("{new_name}-cert.pub"));

        tokio::task::spawn_blocking(move || {
            // Rename private key
            std::fs::rename(&old_private, &new_private).map_err(|e| {
                Error::CommandFailed(format!("failed to rename private key: {e}"))
            })?;

            // Rename public key if it exists
            if old_public.exists()
                && let Err(e) = std::fs::rename(&old_public, &new_public)
            {
                tracing::warn!("failed to rename public key: {e}");
            }

            // Rename certificate if it exists
            if old_cert.exists()
                && let Err(e) = std::fs::rename(&old_cert, &new_cert)
            {
                tracing::warn!("failed to rename certificate: {e}");
            }

            Ok(())
        })
        .await
        .map_err(|e| Error::TaskFailed(format!("rename task failed: {e}")))?
    }

    /// Fix permissions on key files (set private keys to 0o600, public to 0o644).
    pub async fn chmod_fix(&self, key_name: &str) -> Result<()> {
        validate_key_name(key_name)?;

        let private_path = self.paths.ssh_dir().join(key_name);
        if !private_path.exists() {
            return Err(Error::KeyNotFound(key_name.to_owned()));
        }

        let public_path = private_path.with_extension("pub");

        tokio::task::spawn_blocking(move || {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(
                    &private_path,
                    std::fs::Permissions::from_mode(0o600),
                )
                .map_err(|e| Error::CommandFailed(format!("failed to set private key permissions: {e}")))?;

                if public_path.exists()
                    && let Err(e) = std::fs::set_permissions(
                        &public_path,
                        std::fs::Permissions::from_mode(0o644),
                    )
                {
                    tracing::warn!("failed to set public key permissions: {e}");
                }
            }
            #[cfg(not(unix))]
            {
                let _ = (private_path, public_path);
            }
            Ok(())
        })
        .await
        .map_err(|e| Error::TaskFailed(format!("chmod task failed: {e}")))?
    }

    /// Change the passphrase on an existing key (`ssh-keygen -p`).
    ///
    /// If `old_passphrase` is `None`, the key is assumed to be unencrypted.
    /// If `new_passphrase` is `None` or empty, the passphrase is removed.
    ///
    /// # Security
    ///
    /// Passphrases are passed as command-line arguments to `ssh-keygen`, which
    /// makes them briefly visible to other processes via `/proc/<pid>/cmdline`
    /// or `ps`. This is an inherent limitation of the `ssh-keygen` interface.
    pub async fn change_passphrase(
        &self,
        key_path: &std::path::Path,
        old_passphrase: Option<&str>,
        new_passphrase: Option<&str>,
    ) -> Result<()> {
        if !key_path.exists() {
            return Err(Error::KeyNotFound(key_path.display().to_string()));
        }

        let path_str = key_path
            .to_str()
            .ok_or_else(|| Error::CommandFailed("key path is not valid UTF-8".to_owned()))?
            .to_owned();

        let old_pass = old_passphrase.unwrap_or("").to_owned();
        let new_pass = new_passphrase.unwrap_or("").to_owned();

        tokio::task::spawn_blocking(move || {
            let output = duct::cmd(
                "ssh-keygen",
                [
                    "-p",
                    "-f", &path_str,
                    "-P", &old_pass,
                    "-N", &new_pass,
                ],
            )
            .read()
            .map_err(|e| Error::CommandFailed(format!("ssh-keygen -p failed: {e}")))?;

            tracing::debug!("ssh-keygen -p output: {output}");
            Ok(())
        })
        .await
        .map_err(|e| Error::TaskFailed(format!("passphrase change task failed: {e}")))?
    }

    /// Change the comment on an existing key (`ssh-keygen -c`).
    ///
    /// Updates both the private and public key files.
    pub async fn change_comment(
        &self,
        key_path: &std::path::Path,
        new_comment: &str,
        passphrase: Option<&str>,
    ) -> Result<()> {
        if !key_path.exists() {
            return Err(Error::KeyNotFound(key_path.display().to_string()));
        }

        let path_str = key_path
            .to_str()
            .ok_or_else(|| Error::CommandFailed("key path is not valid UTF-8".to_owned()))?
            .to_owned();

        let comment = new_comment.to_owned();
        let pass = passphrase.unwrap_or("").to_owned();

        tokio::task::spawn_blocking(move || {
            let mut args = vec!["-c", "-f", &path_str, "-C", &comment];
            if !pass.is_empty() {
                args.extend(["-P", &pass]);
            }

            let output = duct::cmd("ssh-keygen", &args)
                .read()
                .map_err(|e| Error::CommandFailed(format!("ssh-keygen -c failed: {e}")))?;

            tracing::debug!("ssh-keygen -c output: {output}");
            Ok(())
        })
        .await
        .map_err(|e| Error::TaskFailed(format!("comment change task failed: {e}")))?
    }

    /// Convert a key between OpenSSH and PEM formats.
    ///
    /// - [`KeyFormat::Pem`]: exports the key in PEM format via `ssh-keygen -e -m PEM`.
    /// - [`KeyFormat::OpenSSH`]: imports a PEM-format key to OpenSSH format via `ssh-keygen -i -m PEM`.
    ///
    /// Returns the converted key content as a string.
    pub async fn convert(
        &self,
        key_path: &std::path::Path,
        target_format: KeyFormat,
    ) -> Result<String> {
        if !key_path.exists() {
            return Err(Error::KeyNotFound(key_path.display().to_string()));
        }

        if !crate::runner::tool_exists("ssh-keygen") {
            return Err(Error::ToolNotFound("ssh-keygen".to_owned()));
        }

        let path_str = key_path
            .to_str()
            .ok_or_else(|| Error::CommandFailed("key path is not valid UTF-8".to_owned()))?
            .to_owned();

        tokio::task::spawn_blocking(move || {
            let output = match target_format {
                KeyFormat::Pem => {
                    // Export: read an OpenSSH key and output PEM format.
                    duct::cmd("ssh-keygen", ["-e", "-m", "PEM", "-f", path_str.as_str()])
                        .read()
                        .map_err(|e| Error::CommandFailed(format!("ssh-keygen export failed: {e}")))?
                }
                KeyFormat::OpenSSH => {
                    // Import: read a PEM key and output OpenSSH format.
                    duct::cmd("ssh-keygen", ["-i", "-m", "PEM", "-f", path_str.as_str()])
                        .read()
                        .map_err(|e| Error::CommandFailed(format!("ssh-keygen import failed: {e}")))?
                }
            };
            Ok(output)
        })
        .await
        .map_err(|e| Error::TaskFailed(format!("key conversion task failed: {e}")))?
    }
}

/// Remove a key from the SSH agent.
///
/// This is intentionally non-fatal: the key may not be loaded in the agent,
/// which is a perfectly normal state. Errors are logged but not propagated.
async fn remove_key_from_agent(private_path: &std::path::Path) {
    let Some(path_str) = private_path.to_str().map(str::to_owned) else {
        tracing::warn!("invalid key path for ssh-add, skipping agent removal");
        return;
    };

    let result = tokio::task::spawn_blocking(move || {
        // ssh-add -d removes a key from the agent by path
        duct::cmd("ssh-add", ["-d", path_str.as_str()])
            .read()
            .map_err(|e| {
                tracing::warn!("ssh-add -d failed (key may not be in agent): {e}");
                e
            })
    })
    .await;

    if let Err(e) = result {
        tracing::warn!("ssh-add task failed: {e}");
    }
}

/// Remove `IdentityFile` references from `~/.ssh/config`.
///
/// This is a basic implementation that removes lines containing the key path.
/// Read errors are non-fatal (the config may be unreadable due to permissions).
async fn remove_from_config(paths: &SshPaths, key_name: &str) -> Result<()> {
    // Allocate an owned PathBuf for use inside `spawn_blocking` (requires `'static`).
    let config_path = paths.config_path().to_path_buf();

    if !config_path.exists() {
        return Ok(());
    }

    let key_name_owned = key_name.to_owned();
    let ssh_dir_str = paths
        .ssh_dir()
        .to_str()
        .ok_or_else(|| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("SSH directory path is not valid UTF-8: {}", paths.ssh_dir().display()),
            ))
        })?
        .to_owned();

    tokio::task::spawn_blocking(move || {
        let content = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("cannot read config for cleanup: {e}");
                return Ok(());
            }
        };

        // Remove lines that reference this key as an IdentityFile.
        // Match lines like: IdentityFile ~/.ssh/<key_name> or IdentityFile <full_path>
        let key_pattern_tilde = format!("~/.ssh/{key_name_owned}");
        let key_pattern_abs = format!("{ssh_dir_str}/{key_name_owned}");

        let trailing_newline = content.ends_with('\n');

        let new_content: String = content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                // Extract the keyword (first whitespace-delimited token) and
                // compare case-insensitively.  This avoids matching directives
                // like "IdentityFileSomething" or comments containing the word.
                let keyword = trimmed.split_whitespace().next().unwrap_or("");
                if !keyword.eq_ignore_ascii_case("IdentityFile") {
                    return true;
                }
                // Extract the value (everything after the keyword).
                let value = trimmed[keyword.len()..].trim();
                // Remove quotes if present
                let value = value.trim_matches('"').trim_matches('\'');
                value != key_pattern_tilde && value != key_pattern_abs && value != key_name_owned
            })
            .collect::<Vec<&str>>()
            .join("\n");

        // Preserve trailing newline from the original file
        let final_content = if trailing_newline && !new_content.is_empty() {
            format!("{new_content}\n")
        } else {
            new_content
        };

        if final_content != content {
            // Atomic write: temp file + rename to prevent corruption on crash.
            let parent = config_path.parent().unwrap_or(std::path::Path::new("."));
            let tmp_path = parent.join(format!(
                ".config.tmp.{}.{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            ));
            std::fs::write(&tmp_path, &final_content).map_err(|e| {
                Error::ConfigWriteFailed(format!("failed to write temp config: {e}"))
            })?;
            if let Err(e) = std::fs::rename(&tmp_path, &config_path) {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(Error::ConfigWriteFailed(format!(
                    "failed to rename config: {e}"
                )));
            }
        }

        Ok(())
    })
    .await
    .map_err(|e| Error::TaskFailed(format!("config cleanup task failed: {e}")))?
}

#[cfg(test)]
#[path = "mod.test.rs"]
mod tests;
