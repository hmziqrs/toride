//! Local SSH diagnostic checks.

use std::collections::HashMap;

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use crate::config::ast::{self, ConfigNode};
use crate::doctor::check::{Check, CheckFuture};
use crate::paths::SshPaths;
use crate::types::{Diagnostic, Severity};
use crate::Result;

// ---------------------------------------------------------------------------
// Concrete check structs
// ---------------------------------------------------------------------------

/// Check that `~/.ssh` exists and is a directory.
struct SshDirExists<'a> {
    paths: &'a SshPaths,
}

/// Check that `~/.ssh` has permission mode `0o700` (Unix) or reports ACL info (Windows).
struct SshDirPermissions<'a> {
    paths: &'a SshPaths,
}

/// Check that `~/.ssh/config` exists.
struct ConfigExists<'a> {
    paths: &'a SshPaths,
}

/// Check that `~/.ssh/known_hosts` exists.
struct KnownHostsExists<'a> {
    paths: &'a SshPaths,
}

/// Check that all private key files under `~/.ssh` have mode `0o600` (Unix) or reports ACL info (Windows).
struct PrivateKeyPermissions<'a> {
    paths: &'a SshPaths,
}

/// Check that the SSH agent socket is reachable.
struct AgentAvailable;

/// Check that `ssh-keygen` is available in `PATH`.
struct KeygenAvailable;

/// Check that at least one default key pair exists.
struct DefaultKeyExists<'a> {
    paths: &'a SshPaths,
}

/// Check that `~/.ssh` is owned by the current user (Unix) or reports ACL info (Windows).
struct OwnerCheck<'a> {
    paths: &'a SshPaths,
}

/// Check that `~/.ssh/config` is not group/world writable (Unix) or reports ACL info (Windows).
struct ConfigPermissionsCheck<'a> {
    paths: &'a SshPaths,
}

/// Check that `~/.ssh/authorized_keys` has mode `0o600` or `0o644` (Unix) or reports ACL info (Windows).
struct AuthorizedKeysPermissionsCheck<'a> {
    paths: &'a SshPaths,
}

/// Check that every `id_*` private key has a matching `.pub` file.
struct PublicKeyPairsCheck<'a> {
    paths: &'a SshPaths,
}

/// Check that `IdentityFile` paths referenced in config actually exist.
struct IdentityFileExistsCheck<'a> {
    paths: &'a SshPaths,
}

/// Detect duplicate `Host` blocks with the same pattern.
struct DuplicateHostCheck<'a> {
    paths: &'a SshPaths,
}

/// Detect `Host *` appearing before specific `Host` blocks.
struct HostStarPlacementCheck<'a> {
    paths: &'a SshPaths,
}

/// Check for deprecated SSHv1 key files (`~/.ssh/identity`).
struct SshV1KeyCheck<'a> {
    paths: &'a SshPaths,
}

// ---------------------------------------------------------------------------
// Check implementations
// ---------------------------------------------------------------------------

impl Check for SshDirExists<'_> {
    fn id(&self) -> &'static str {
        "ssh_dir_exists"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(
        &self,
    ) -> CheckFuture<'_>
    {
        let ssh_dir = self.paths.ssh_dir().to_path_buf();
        Box::pin(async move {
            let meta = tokio::fs::metadata(&ssh_dir).await;
            match meta {
                Ok(m) if m.is_dir() => Ok(vec![Diagnostic {
                    id: "ssh_dir_exists",
                    severity: Severity::Ok,
                    message: format!("{} exists and is a directory", ssh_dir.display()),
                    hint: None,
                    module: "local",
                }]),
                Ok(_) => Ok(vec![Diagnostic {
                    id: "ssh_dir_exists",
                    severity: Severity::Error,
                    message: format!("{} exists but is not a directory", ssh_dir.display()),
                    hint: Some("Remove the file and run `mkdir -p ~/.ssh`".into()),
                    module: "local",
                }]),
                Err(e) => Ok(vec![Diagnostic {
                    id: "ssh_dir_exists",
                    severity: Severity::Warning,
                    message: format!("{} does not exist: {e}", ssh_dir.display()),
                    hint: Some("Run `mkdir -p ~/.ssh && chmod 700 ~/.ssh`".into()),
                    module: "local",
                }]),
            }
        })
    }
}

#[cfg(unix)]
impl Check for SshDirPermissions<'_> {
    fn id(&self) -> &'static str {
        "ssh_dir_permissions"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(
        &self,
    ) -> CheckFuture<'_>
    {
        let ssh_dir = self.paths.ssh_dir().to_path_buf();
        Box::pin(async move {
            let meta = tokio::fs::metadata(&ssh_dir).await;
            match meta {
                Ok(m) => {
                    let mode = m.permissions().mode() & 0o777;
                    if mode == 0o700 {
                        Ok(vec![Diagnostic {
                            id: "ssh_dir_permissions",
                            severity: Severity::Ok,
                            message: format!(
                                "{} has correct permissions ({:o})",
                                ssh_dir.display(),
                                mode
                            ),
                            hint: None,
                            module: "local",
                        }])
                    } else {
                        Ok(vec![Diagnostic {
                            id: "ssh_dir_permissions",
                            severity: Severity::Error,
                            message: format!(
                                "{} has overly permissive permissions ({:o}), expected 700",
                                ssh_dir.display(),
                                mode
                            ),
                            hint: Some("Run `chmod 700 ~/.ssh`".into()),
                            module: "local",
                        }])
                    }
                }
                Err(e) => Ok(vec![Diagnostic {
                    id: "ssh_dir_permissions",
                    severity: Severity::Warning,
                    message: format!(
                        "Cannot check permissions: {}: {e}",
                        ssh_dir.display()
                    ),
                    hint: Some("Run `mkdir -p ~/.ssh && chmod 700 ~/.ssh`".into()),
                    module: "local",
                }]),
            }
        })
    }
}

impl Check for ConfigExists<'_> {
    fn id(&self) -> &'static str {
        "config_exists"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(
        &self,
    ) -> CheckFuture<'_>
    {
        let config_path = self.paths.config_path();
        Box::pin(async move {
            match tokio::fs::metadata(&config_path).await {
                Ok(_) => Ok(vec![Diagnostic {
                    id: "config_exists",
                    severity: Severity::Ok,
                    message: format!("{} exists", config_path.display()),
                    hint: None,
                    module: "local",
                }]),
                Err(_) => Ok(vec![Diagnostic {
                    id: "config_exists",
                    severity: Severity::Info,
                    message: format!("{} does not exist", config_path.display()),
                    hint: Some(
                        "An SSH config is optional but recommended. Create it with `touch ~/.ssh/config`".into(),
                    ),
                    module: "local",
                }]),
            }
        })
    }
}

impl Check for KnownHostsExists<'_> {
    fn id(&self) -> &'static str {
        "known_hosts_exists"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(
        &self,
    ) -> CheckFuture<'_>
    {
        let kh_path = self.paths.known_hosts_path();
        Box::pin(async move {
            match tokio::fs::metadata(&kh_path).await {
                Ok(_) => Ok(vec![Diagnostic {
                    id: "known_hosts_exists",
                    severity: Severity::Ok,
                    message: format!("{} exists", kh_path.display()),
                    hint: None,
                    module: "local",
                }]),
                Err(_) => Ok(vec![Diagnostic {
                    id: "known_hosts_exists",
                    severity: Severity::Warning,
                    message: format!("{} does not exist", kh_path.display()),
                    hint: Some(
                        "Without a known_hosts file you will be prompted to verify every new host. Run `touch ~/.ssh/known_hosts`".into(),
                    ),
                    module: "local",
                }]),
            }
        })
    }
}

#[cfg(unix)]
impl Check for PrivateKeyPermissions<'_> {
    fn id(&self) -> &'static str {
        "private_key_permissions"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(
        &self,
    ) -> CheckFuture<'_>
    {
        let ssh_dir = self.paths.ssh_dir().to_path_buf();
        Box::pin(async move {
            let mut diagnostics = Vec::new();
            let Ok(mut read_dir) = tokio::fs::read_dir(&ssh_dir).await else {
                return Ok(vec![Diagnostic {
                    id: "private_key_permissions",
                    severity: Severity::Warning,
                    message: format!(
                        "Cannot scan {}: directory does not exist",
                        ssh_dir.display()
                    ),
                    hint: None,
                    module: "local",
                }]);
            };

            while let Some(entry) = read_dir.next_entry().await? {
                let name = entry.file_name();
                let name_lossy = name.to_string_lossy();

                // Skip public keys, certificates, config files, and dotfiles.
                if name_lossy.ends_with(".pub")
                    || name_lossy.ends_with("-cert.pub")
                    || name_lossy == "config"
                    || name_lossy == "known_hosts"
                    || name_lossy == "authorized_keys"
                    || name_lossy == "authorized_keys2"
                    || name_lossy.starts_with('.')
                {
                    continue;
                }

                let path = entry.path();
                let Ok(meta) = tokio::fs::metadata(&path).await else {
                    continue;
                };

                // Only check regular files.
                if !meta.is_file() {
                    continue;
                }

                // Read only the first 4 KB to detect private key markers without
                // loading potentially large files into memory.
                {
                    use tokio::io::AsyncReadExt;
                    let Ok(mut file) = tokio::fs::File::open(&path).await else {
                        continue;
                    };
                    let mut buf = [0u8; 4096];
                    let Ok(n) = file.read(&mut buf).await else {
                        continue;
                    };
                    // `from_utf8_lossy` returns `Cow<str>` so no allocation
                    // occurs when the header is valid UTF-8 (always true for PEM).
                    if n < 8 || !String::from_utf8_lossy(&buf[..n]).contains("PRIVATE KEY") {
                        continue;
                    }
                }

                let mode = meta.permissions().mode() & 0o777;
                if mode == 0o600 {
                    diagnostics.push(Diagnostic {
                        id: "private_key_permissions",
                        severity: Severity::Ok,
                        message: format!(
                            "{} has correct permissions ({:o})",
                            path.display(),
                            mode
                        ),
                        hint: None,
                        module: "local",
                    });
                } else {
                    diagnostics.push(Diagnostic {
                        id: "private_key_permissions",
                        severity: Severity::Error,
                        message: format!(
                            "{} has overly permissive permissions ({:o}), expected 600",
                            path.display(),
                            mode
                        ),
                        hint: Some(format!("Run `chmod 600 {}`", path.display())),
                        module: "local",
                    });
                }
            }

            if diagnostics.is_empty() {
                diagnostics.push(Diagnostic {
                    id: "private_key_permissions",
                    severity: Severity::Info,
                    message: "No private key files found in ~/.ssh".into(),
                    hint: None,
                    module: "local",
                });
            }

            Ok(diagnostics)
        })
    }
}

impl Check for AgentAvailable {
    fn id(&self) -> &'static str {
        "agent_available"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(
        &self,
    ) -> CheckFuture<'_>
    {
        Box::pin(async move {
            match std::env::var("SSH_AUTH_SOCK") {
                Ok(sock) if !sock.is_empty() => {
                    let sock_path = std::path::PathBuf::from(&sock);
                    match tokio::fs::metadata(&sock_path).await {
                        Ok(_) => Ok(vec![Diagnostic {
                            id: "agent_available",
                            severity: Severity::Ok,
                            message: "SSH agent is reachable via $SSH_AUTH_SOCK".into(),
                            hint: None,
                            module: "local",
                        }]),
                        Err(_) => Ok(vec![Diagnostic {
                            id: "agent_available",
                            severity: Severity::Warning,
                            message: format!(
                                "$SSH_AUTH_SOCK is set to {sock} but the socket does not exist",
                            ),
                            hint: Some(
                                "Start the SSH agent: `eval $(ssh-agent -s)`".into(),
                            ),
                            module: "local",
                        }]),
                    }
                }
                _ => Ok(vec![Diagnostic {
                    id: "agent_available",
                    severity: Severity::Warning,
                    message: "$SSH_AUTH_SOCK is not set — SSH agent may not be running".into(),
                    hint: Some(
                        "Start the SSH agent: `eval $(ssh-agent -s)`".into(),
                    ),
                    module: "local",
                }]),
            }
        })
    }
}

impl Check for KeygenAvailable {
    fn id(&self) -> &'static str {
        "keygen_available"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(
        &self,
    ) -> CheckFuture<'_>
    {
        Box::pin(async move {
            if crate::runner::tool_exists("ssh-keygen") {
                Ok(vec![Diagnostic {
                    id: "keygen_available",
                    severity: Severity::Ok,
                    message: "ssh-keygen is available in PATH".into(),
                    hint: None,
                    module: "local",
                }])
            } else {
                Ok(vec![Diagnostic {
                    id: "keygen_available",
                    severity: Severity::Error,
                    message: "ssh-keygen is not found in PATH".into(),
                    hint: Some(
                        "Install OpenSSH: `brew install openssh` (macOS) or `sudo apt install openssh-client` (Linux)".into(),
                    ),
                    module: "local",
                }])
            }
        })
    }
}

impl Check for DefaultKeyExists<'_> {
    fn id(&self) -> &'static str {
        "default_key_exists"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(
        &self,
    ) -> CheckFuture<'_>
    {
        let ssh_dir = self.paths.ssh_dir().to_path_buf();
        Box::pin(async move {
            let mut found = Vec::new();
            for name in SshPaths::default_key_names() {
                let key_path = ssh_dir.join(name);
                if tokio::fs::metadata(&key_path).await.is_ok() {
                    found.push(*name);
                }
            }

            if found.is_empty() {
                Ok(vec![Diagnostic {
                    id: "default_key_exists",
                    severity: Severity::Warning,
                    message: "No default SSH key found (checked id_rsa, id_ed25519, etc.)".into(),
                    hint: Some(
                        "Generate a key: `ssh-keygen -t ed25519 -C \"your@email.com\"`".into(),
                    ),
                    module: "local",
                }])
            } else {
                Ok(vec![Diagnostic {
                    id: "default_key_exists",
                    severity: Severity::Ok,
                    message: format!("Default key(s) found: {}", found.join(", ")),
                    hint: None,
                    module: "local",
                }])
            }
        })
    }
}

// ---------------------------------------------------------------------------
// OwnerCheck
// ---------------------------------------------------------------------------

#[cfg(unix)]
impl Check for OwnerCheck<'_> {
    fn id(&self) -> &'static str {
        "owner_check"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(&self) -> CheckFuture<'_> {
        let ssh_dir = self.paths.ssh_dir().to_path_buf();
        Box::pin(async move {
            let meta = tokio::fs::metadata(&ssh_dir).await;
            match meta {
                Ok(m) => {
                    let file_uid = m.uid();
                    let current_uid = unsafe { libc::getuid() };
                    if file_uid == current_uid {
                        Ok(vec![Diagnostic {
                            id: "owner_check",
                            severity: Severity::Ok,
                            message: format!(
                                "{} is owned by current user (uid {})",
                                ssh_dir.display(),
                                current_uid
                            ),
                            hint: None,
                            module: "local",
                        }])
                    } else {
                        Ok(vec![Diagnostic {
                            id: "owner_check",
                            severity: Severity::Error,
                            message: format!(
                                "{} is owned by uid {} but current user is uid {}",
                                ssh_dir.display(),
                                file_uid,
                                current_uid
                            ),
                            hint: Some(format!(
                                "Run `sudo chown -R $(id -u) {}`",
                                ssh_dir.display()
                            )),
                            module: "local",
                        }])
                    }
                }
                Err(e) => Ok(vec![Diagnostic {
                    id: "owner_check",
                    severity: Severity::Warning,
                    message: format!(
                        "Cannot check ownership: {}: {e}",
                        ssh_dir.display()
                    ),
                    hint: Some("Run `mkdir -p ~/.ssh && chmod 700 ~/.ssh`".into()),
                    module: "local",
                }]),
            }
        })
    }
}

// ---------------------------------------------------------------------------
// ConfigPermissionsCheck
// ---------------------------------------------------------------------------

#[cfg(unix)]
impl Check for ConfigPermissionsCheck<'_> {
    fn id(&self) -> &'static str {
        "config_permissions"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(&self) -> CheckFuture<'_> {
        let config_path = self.paths.config_path().to_path_buf();
        Box::pin(async move {
            let meta = tokio::fs::metadata(&config_path).await;
            match meta {
                Ok(m) => {
                    let mode = m.permissions().mode() & 0o777;
                    if mode & 0o022 == 0 {
                        Ok(vec![Diagnostic {
                            id: "config_permissions",
                            severity: Severity::Ok,
                            message: format!(
                                "{} is not group/world writable ({:o})",
                                config_path.display(),
                                mode
                            ),
                            hint: None,
                            module: "local",
                        }])
                    } else {
                        Ok(vec![Diagnostic {
                            id: "config_permissions",
                            severity: Severity::Error,
                            message: format!(
                                "{} is group/world writable ({:o}), expected no group/world write bits",
                                config_path.display(),
                                mode
                            ),
                            hint: Some(format!("Run `chmod 600 {}`", config_path.display())),
                            module: "local",
                        }])
                    }
                }
                Err(_) => Ok(vec![Diagnostic {
                    id: "config_permissions",
                    severity: Severity::Info,
                    message: format!(
                        "Cannot check permissions: {} does not exist",
                        config_path.display()
                    ),
                    hint: None,
                    module: "local",
                }]),
            }
        })
    }
}

// ---------------------------------------------------------------------------
// AuthorizedKeysPermissionsCheck
// ---------------------------------------------------------------------------

#[cfg(unix)]
impl Check for AuthorizedKeysPermissionsCheck<'_> {
    fn id(&self) -> &'static str {
        "authorized_keys_permissions"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(&self) -> CheckFuture<'_> {
        let ak_path = self.paths.authorized_keys_path().to_path_buf();
        Box::pin(async move {
            let meta = tokio::fs::metadata(&ak_path).await;
            match meta {
                Ok(m) => {
                    let mode = m.permissions().mode() & 0o777;
                    if mode == 0o600 || mode == 0o644 {
                        Ok(vec![Diagnostic {
                            id: "authorized_keys_permissions",
                            severity: Severity::Ok,
                            message: format!(
                                "{} has acceptable permissions ({:o})",
                                ak_path.display(),
                                mode
                            ),
                            hint: None,
                            module: "local",
                        }])
                    } else {
                        Ok(vec![Diagnostic {
                            id: "authorized_keys_permissions",
                            severity: Severity::Error,
                            message: format!(
                                "{} has permissions {:o}, expected 600 or 644",
                                ak_path.display(),
                                mode
                            ),
                            hint: Some(format!("Run `chmod 600 {}`", ak_path.display())),
                            module: "local",
                        }])
                    }
                }
                Err(_) => Ok(vec![Diagnostic {
                    id: "authorized_keys_permissions",
                    severity: Severity::Info,
                    message: format!(
                        "Cannot check permissions: {} does not exist",
                        ak_path.display()
                    ),
                    hint: None,
                    module: "local",
                }]),
            }
        })
    }
}

// ---------------------------------------------------------------------------
// Non-Unix (Windows) stubs for permission checks
// ---------------------------------------------------------------------------

#[cfg(not(unix))]
impl Check for SshDirPermissions<'_> {
    fn id(&self) -> &'static str {
        "ssh_dir_permissions"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(&self) -> CheckFuture<'_> {
        let ssh_dir = self.paths.ssh_dir().to_path_buf();
        Box::pin(async move {
            Ok(vec![Diagnostic {
                id: "ssh_dir_permissions",
                severity: Severity::Info,
                message: format!(
                    "Permission check for {} skipped (not supported on this platform)",
                    ssh_dir.display()
                ),
                hint: Some(
                    "On Windows, verify ACLs with: icacls \"%USERPROFILE%\\.ssh\"".into(),
                ),
                module: "local",
            }])
        })
    }
}

#[cfg(not(unix))]
impl Check for PrivateKeyPermissions<'_> {
    fn id(&self) -> &'static str {
        "private_key_permissions"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(&self) -> CheckFuture<'_> {
        let ssh_dir = self.paths.ssh_dir().to_path_buf();
        Box::pin(async move {
            Ok(vec![Diagnostic {
                id: "private_key_permissions",
                severity: Severity::Info,
                message: format!(
                    "Private key permission check for {} skipped (not supported on this platform)",
                    ssh_dir.display()
                ),
                hint: Some(
                    "On Windows, restrict access to private key files with: icacls <keyfile> /inheritance:r /grant:r \"%USERNAME%:R\"".into(),
                ),
                module: "local",
            }])
        })
    }
}

#[cfg(not(unix))]
impl Check for OwnerCheck<'_> {
    fn id(&self) -> &'static str {
        "owner_check"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(&self) -> CheckFuture<'_> {
        let ssh_dir = self.paths.ssh_dir().to_path_buf();
        Box::pin(async move {
            Ok(vec![Diagnostic {
                id: "owner_check",
                severity: Severity::Info,
                message: format!(
                    "Ownership check for {} skipped (not supported on this platform)",
                    ssh_dir.display()
                ),
                hint: Some(
                    "On Windows, verify ownership with: icacls \"%USERPROFILE%\\.ssh\"".into(),
                ),
                module: "local",
            }])
        })
    }
}

#[cfg(not(unix))]
impl Check for ConfigPermissionsCheck<'_> {
    fn id(&self) -> &'static str {
        "config_permissions"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(&self) -> CheckFuture<'_> {
        let config_path = self.paths.config_path().to_path_buf();
        Box::pin(async move {
            Ok(vec![Diagnostic {
                id: "config_permissions",
                severity: Severity::Info,
                message: format!(
                    "Permission check for {} skipped (not supported on this platform)",
                    config_path.display()
                ),
                hint: Some(
                    "On Windows, restrict access to config with: icacls <configfile> /inheritance:r /grant:r \"%USERNAME%:F\"".into(),
                ),
                module: "local",
            }])
        })
    }
}

#[cfg(not(unix))]
impl Check for AuthorizedKeysPermissionsCheck<'_> {
    fn id(&self) -> &'static str {
        "authorized_keys_permissions"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(&self) -> CheckFuture<'_> {
        let ak_path = self.paths.authorized_keys_path().to_path_buf();
        Box::pin(async move {
            Ok(vec![Diagnostic {
                id: "authorized_keys_permissions",
                severity: Severity::Info,
                message: format!(
                    "Permission check for {} skipped (not supported on this platform)",
                    ak_path.display()
                ),
                hint: Some(
                    "On Windows, verify ACLs with: icacls \"%USERPROFILE%\\.ssh\\authorized_keys\"".into(),
                ),
                module: "local",
            }])
        })
    }
}

// ---------------------------------------------------------------------------
// PlatformCheck
// ---------------------------------------------------------------------------

/// Report platform information: OS, architecture, SSH version, and agent type.
struct PlatformCheck;

impl Check for PlatformCheck {
    fn id(&self) -> &'static str {
        "platform_check"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(&self) -> CheckFuture<'_> {
        Box::pin(async move {
            let mut diagnostics = Vec::new();

            // Report OS and architecture.
            diagnostics.push(Diagnostic {
                id: "platform_os",
                severity: Severity::Info,
                message: format!(
                    "Operating system: {} ({})",
                    std::env::consts::OS,
                    std::env::consts::ARCH
                ),
                hint: None,
                module: "local",
            });

            // Detect SSH agent type.
            #[cfg(unix)]
            {
                match std::env::var("SSH_AUTH_SOCK") {
                    Ok(sock) if !sock.is_empty() => {
                        diagnostics.push(Diagnostic {
                            id: "platform_agent_type",
                            severity: Severity::Info,
                            message: format!("SSH agent: Unix socket at {sock}"),
                            hint: None,
                            module: "local",
                        });
                    }
                    _ => {
                        diagnostics.push(Diagnostic {
                            id: "platform_agent_type",
                            severity: Severity::Info,
                            message: "SSH agent: not detected ($SSH_AUTH_SOCK not set)".into(),
                            hint: Some("Start the agent: `eval $(ssh-agent -s)`".into()),
                            module: "local",
                        });
                    }
                }
            }

            #[cfg(windows)]
            {
                if std::env::var("SSH_AUTH_SOCK").is_ok()
                    || std::env::var("SSH_AGENT_PID").is_ok()
                {
                    diagnostics.push(Diagnostic {
                        id: "platform_agent_type",
                        severity: Severity::Info,
                        message: "SSH agent: OpenSSH agent (Windows)".into(),
                        hint: None,
                        module: "local",
                    });
                } else {
                    diagnostics.push(Diagnostic {
                        id: "platform_agent_type",
                        severity: Severity::Info,
                        message: "SSH agent: not detected".into(),
                        hint: Some(
                            "Enable the OpenSSH agent: \
                             `Get-Service ssh-agent | Set-Service -StartupType Automatic`"
                                .into(),
                        ),
                        module: "local",
                    });
                }
            }

            // Run `ssh -V` to get the SSH client version.
            // `ssh -V` prints to stderr.
            let version_output = tokio::task::spawn_blocking(|| {
                duct::cmd("ssh", ["-V"])
                    .stderr_to_stdout()
                    .read()
                    .ok()
            })
            .await
            .ok()
            .flatten();

            match version_output {
                Some(v) => {
                    let version = v.trim().to_owned();
                    diagnostics.push(Diagnostic {
                        id: "platform_ssh_version",
                        severity: Severity::Ok,
                        message: format!("SSH client: {version}"),
                        hint: None,
                        module: "local",
                    });
                }
                None => {
                    diagnostics.push(Diagnostic {
                        id: "platform_ssh_version",
                        severity: Severity::Warning,
                        message: "SSH client not found in PATH".into(),
                        hint: Some(
                            "Install OpenSSH: \
                             `brew install openssh` (macOS), \
                             `sudo apt install openssh-client` (Linux), \
                             or enable the OpenSSH Client feature (Windows)"
                                .into(),
                        ),
                        module: "local",
                    });
                }
            }

            Ok(diagnostics)
        })
    }
}

// ---------------------------------------------------------------------------
// PublicKeyPairsCheck
// ---------------------------------------------------------------------------

impl Check for PublicKeyPairsCheck<'_> {
    fn id(&self) -> &'static str {
        "public_key_pairs"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(&self) -> CheckFuture<'_> {
        let ssh_dir = self.paths.ssh_dir().to_path_buf();
        Box::pin(async move {
            let mut diagnostics = Vec::new();
            let Ok(mut read_dir) = tokio::fs::read_dir(&ssh_dir).await else {
                return Ok(vec![Diagnostic {
                    id: "public_key_pairs",
                    severity: Severity::Warning,
                    message: format!(
                        "Cannot scan {}: directory does not exist",
                        ssh_dir.display()
                    ),
                    hint: None,
                    module: "local",
                }]);
            };

            let mut private_keys = Vec::new();
            while let Some(entry) = read_dir.next_entry().await? {
                let name = entry.file_name();
                let name_str = name.to_string_lossy().to_string();

                // Match id_* private keys (not .pub, not -cert.pub).
                if name_str.starts_with("id_")
                    && !name_str.to_lowercase().ends_with(".pub")
                    && !name_str.to_lowercase().ends_with("-cert.pub")
                {
                    let path = entry.path();
                    let Ok(meta) = tokio::fs::metadata(&path).await else {
                        continue;
                    };
                    if meta.is_file() {
                        private_keys.push((name_str, path));
                    }
                }
            }

            if private_keys.is_empty() {
                return Ok(vec![Diagnostic {
                    id: "public_key_pairs",
                    severity: Severity::Info,
                    message: "No id_* private keys found in ~/.ssh".into(),
                    hint: None,
                    module: "local",
                }]);
            }

            for (name, path) in &private_keys {
                let pub_path = path.with_file_name(format!("{name}.pub"));
                if tokio::fs::metadata(&pub_path).await.is_ok() {
                    diagnostics.push(Diagnostic {
                        id: "public_key_pairs",
                        severity: Severity::Ok,
                        message: format!("{} has matching public key", path.display()),
                        hint: None,
                        module: "local",
                    });
                } else {
                    diagnostics.push(Diagnostic {
                        id: "public_key_pairs",
                        severity: Severity::Warning,
                        message: format!(
                            "{} has no matching public key file ({}.pub)",
                            path.display(),
                            name
                        ),
                        hint: Some(format!(
                            "Run `ssh-keygen -y -f {} > {}.pub`",
                            path.display(),
                            path.display()
                        )),
                        module: "local",
                    });
                }
            }

            Ok(diagnostics)
        })
    }
}

// ---------------------------------------------------------------------------
// IdentityFileExistsCheck
// ---------------------------------------------------------------------------

impl Check for IdentityFileExistsCheck<'_> {
    fn id(&self) -> &'static str {
        "identity_file_exists"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(&self) -> CheckFuture<'_> {
        let config_path = self.paths.config_path().to_path_buf();
        let ssh_dir = self.paths.ssh_dir().to_path_buf();
        Box::pin(async move {
            let Ok(content) = tokio::fs::read_to_string(&config_path).await else {
                return Ok(vec![Diagnostic {
                    id: "identity_file_exists",
                    severity: Severity::Info,
                    message: format!(
                        "Cannot check IdentityFile directives: {} does not exist",
                        config_path.display()
                    ),
                    hint: None,
                    module: "local",
                }]);
            };

            let ast = ast::parse(&content);
            let mut identity_files: Vec<String> = Vec::new();

            // Collect IdentityFile directives from top-level and Host blocks.
            for node in &ast.nodes {
                match node {
                    ConfigNode::Directive {
                        keyword, value, ..
                    } if keyword.eq_ignore_ascii_case("IdentityFile") => {
                        identity_files.push(value.clone());
                    }
                    ConfigNode::HostBlock { nodes, .. } => {
                        for child in nodes {
                            if let ConfigNode::Directive {
                                keyword, value, ..
                            } = child
                                && keyword.eq_ignore_ascii_case("IdentityFile")
                            {
                                identity_files.push(value.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }

            if identity_files.is_empty() {
                return Ok(vec![Diagnostic {
                    id: "identity_file_exists",
                    severity: Severity::Info,
                    message: "No IdentityFile directives found in config".into(),
                    hint: None,
                    module: "local",
                }]);
            }

            let mut diagnostics = Vec::new();
            for raw_path in &identity_files {
                // Expand ~ to home directory.
                let expanded = if let Some(rest) = raw_path.strip_prefix("~/") {
                    dirs::home_dir().map_or_else(
                        || std::path::PathBuf::from(raw_path),
                        |home| home.join(rest),
                    )
                } else if let Some(rest) = raw_path.strip_prefix('~') {
                    dirs::home_dir().map_or_else(
                        || std::path::PathBuf::from(raw_path),
                        |home| home.join(rest),
                    )
                } else if std::path::Path::new(raw_path).is_relative() {
                    ssh_dir.join(raw_path)
                } else {
                    std::path::PathBuf::from(raw_path)
                };

                if tokio::fs::metadata(&expanded).await.is_ok() {
                    diagnostics.push(Diagnostic {
                        id: "identity_file_exists",
                        severity: Severity::Ok,
                        message: format!(
                            "IdentityFile {raw_path} exists ({})",
                            expanded.display()
                        ),
                        hint: None,
                        module: "local",
                    });
                } else {
                    diagnostics.push(Diagnostic {
                        id: "identity_file_exists",
                        severity: Severity::Warning,
                        message: format!(
                            "IdentityFile {raw_path} does not exist ({})",
                            expanded.display()
                        ),
                        hint: Some(format!(
                            "Generate the missing key or update the config entry for {raw_path}",
                        )),
                        module: "local",
                    });
                }
            }

            Ok(diagnostics)
        })
    }
}

// ---------------------------------------------------------------------------
// DuplicateHostCheck
// ---------------------------------------------------------------------------

impl Check for DuplicateHostCheck<'_> {
    fn id(&self) -> &'static str {
        "duplicate_host"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(&self) -> CheckFuture<'_> {
        let config_path = self.paths.config_path().to_path_buf();
        Box::pin(async move {
            let Ok(content) = tokio::fs::read_to_string(&config_path).await else {
                return Ok(vec![Diagnostic {
                    id: "duplicate_host",
                    severity: Severity::Info,
                    message: format!(
                        "Cannot check for duplicate hosts: {} does not exist",
                        config_path.display()
                    ),
                    hint: None,
                    module: "local",
                }]);
            };

            let ast = ast::parse(&content);

            // Track each pattern and the first Host block header it appeared in.
            let mut seen: HashMap<String, String> = HashMap::new();
            let mut duplicates: Vec<(String, String, String)> = Vec::new();

            for node in &ast.nodes {
                if let ConfigNode::HostBlock {
                    header, patterns, ..
                } = node
                {
                    for pat in patterns {
                        if pat == "*" {
                            // Skip wildcard for duplicate detection.
                            continue;
                        }
                        if let Some(first_header) = seen.get(pat) {
                            duplicates.push((
                                pat.clone(),
                                first_header.clone(),
                                header.clone(),
                            ));
                        } else {
                            seen.insert(pat.clone(), header.clone());
                        }
                    }
                }
            }

            if duplicates.is_empty() {
                Ok(vec![Diagnostic {
                    id: "duplicate_host",
                    severity: Severity::Ok,
                    message: "No duplicate Host patterns found in config".into(),
                    hint: None,
                    module: "local",
                }])
            } else {
                let mut diagnostics = Vec::new();
                for (pattern, first, second) in &duplicates {
                    diagnostics.push(Diagnostic {
                        id: "duplicate_host",
                        severity: Severity::Warning,
                        message: format!(
                            "Host pattern '{pattern}' appears in multiple blocks: '{first}' and '{second}'",
                        ),
                        hint: Some(format!(
                            "Merge or remove the duplicate entry for '{pattern}'",
                        )),
                        module: "local",
                    });
                }
                Ok(diagnostics)
            }
        })
    }
}

// ---------------------------------------------------------------------------
// HostStarPlacementCheck
// ---------------------------------------------------------------------------

impl Check for HostStarPlacementCheck<'_> {
    fn id(&self) -> &'static str {
        "host_star_placement"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(&self) -> CheckFuture<'_> {
        let config_path = self.paths.config_path().to_path_buf();
        Box::pin(async move {
            let Ok(content) = tokio::fs::read_to_string(&config_path).await else {
                return Ok(vec![Diagnostic {
                    id: "host_star_placement",
                    severity: Severity::Info,
                    message: format!(
                        "Cannot check Host * placement: {} does not exist",
                        config_path.display()
                    ),
                    hint: None,
                    module: "local",
                }]);
            };

            let ast = ast::parse(&content);

            // Find the index of the first Host * and the last specific Host block.
            let mut star_index: Option<usize> = None;
            let mut last_specific_index: Option<usize> = None;

            for (i, node) in ast.nodes.iter().enumerate() {
                if let ConfigNode::HostBlock { patterns, .. } = node {
                    if patterns.iter().any(|p| p == "*") {
                        if star_index.is_none() {
                            star_index = Some(i);
                        }
                    } else if !patterns.is_empty() {
                        last_specific_index = Some(i);
                    }
                }
            }

            match (star_index, last_specific_index) {
                (Some(star), Some(last)) if star < last => Ok(vec![Diagnostic {
                    id: "host_star_placement",
                    severity: Severity::Warning,
                    message:
                        "'Host *' appears before specific Host blocks; \
                         later Host blocks cannot override its defaults"
                            .into(),
                    hint: Some(
                        "Move 'Host *' to the end of the config file so specific \
                         blocks take precedence"
                            .into(),
                    ),
                    module: "local",
                }]),
                _ => Ok(vec![Diagnostic {
                    id: "host_star_placement",
                    severity: Severity::Ok,
                    message: "'Host *' placement is correct (after specific Host blocks, or absent)".into(),
                    hint: None,
                    module: "local",
                }]),
            }
        })
    }
}

// ---------------------------------------------------------------------------
// SshV1KeyCheck
// ---------------------------------------------------------------------------

impl Check for SshV1KeyCheck<'_> {
    fn id(&self) -> &'static str {
        "ssh_v1_key"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(&self) -> CheckFuture<'_> {
        let ssh_dir = self.paths.ssh_dir().to_path_buf();
        Box::pin(async move {
            let identity_path = ssh_dir.join("identity");
            let identity_pub_path = ssh_dir.join("identity.pub");

            let has_private = tokio::fs::metadata(&identity_path).await.is_ok();
            let has_public = tokio::fs::metadata(&identity_pub_path).await.is_ok();

            if has_private || has_public {
                let mut found = Vec::new();
                if has_private {
                    found.push(identity_path.display().to_string());
                }
                if has_public {
                    found.push(identity_pub_path.display().to_string());
                }
                Ok(vec![Diagnostic {
                    id: "ssh_v1_key",
                    severity: Severity::Warning,
                    message: format!(
                        "Deprecated SSHv1 key file(s) found: {}",
                        found.join(", ")
                    ),
                    hint: Some(
                        "SSHv1 keys are insecure. Remove them and use Ed25519 or RSA keys instead: \
                         `rm ~/.ssh/identity ~/.ssh/identity.pub`"
                            .into(),
                    ),
                    module: "local",
                }])
            } else {
                Ok(vec![Diagnostic {
                    id: "ssh_v1_key",
                    severity: Severity::Ok,
                    message: "No deprecated SSHv1 key files found".into(),
                    hint: None,
                    module: "local",
                }])
            }
        })
    }
}

// ---------------------------------------------------------------------------
// IdentityFilePubCheck — warn if IdentityFile points to a .pub file
// ---------------------------------------------------------------------------

/// Check that `IdentityFile` directives don't accidentally point to `.pub` files.
struct IdentityFilePubCheck<'a> {
    paths: &'a SshPaths,
}

impl Check for IdentityFilePubCheck<'_> {
    fn id(&self) -> &'static str {
        "identity_file_pub"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(&self) -> CheckFuture<'_> {
        let config_path = self.paths.config_path().to_path_buf();
        Box::pin(async move {
            let Ok(content) = tokio::fs::read_to_string(&config_path).await else {
                return Ok(vec![]);
            };

            let ast = ast::parse(&content);
            let mut diagnostics = Vec::new();

            let check_value = |raw: &str, diags: &mut Vec<Diagnostic>| {
                let path = std::path::Path::new(raw);
                let is_pub = path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("pub"));
                let is_cert = raw.to_lowercase().ends_with("-cert.pub");
                if is_pub || is_cert {
                    diags.push(Diagnostic {
                        id: "identity_file_pub",
                        severity: Severity::Warning,
                        message: format!(
                            "IdentityFile {raw} points to a public key file; \
                             IdentityFile should reference the private key"
                        ),
                        hint: Some("Change to the private key path (remove .pub suffix)".into()),
                        module: "local",
                    });
                }
            };

            for node in &ast.nodes {
                match node {
                    ConfigNode::Directive {
                        keyword, value, ..
                    } if keyword.eq_ignore_ascii_case("IdentityFile") => {
                        check_value(value, &mut diagnostics);
                    }
                    ConfigNode::HostBlock { nodes, .. } => {
                        for child in nodes {
                            if let ConfigNode::Directive {
                                keyword, value, ..
                            } = child
                                && keyword.eq_ignore_ascii_case("IdentityFile")
                            {
                                check_value(value, &mut diagnostics);
                            }
                        }
                    }
                    _ => {}
                }
            }

            if diagnostics.is_empty() {
                Ok(vec![Diagnostic {
                    id: "identity_file_pub",
                    severity: Severity::Ok,
                    message: "No IdentityFile directives point to .pub files".into(),
                    hint: None,
                    module: "local",
                }])
            } else {
                Ok(diagnostics)
            }
        })
    }
}

// ---------------------------------------------------------------------------
// IdentitiesOnlyCheck — warn when multi-key hosts lack IdentitiesOnly
// ---------------------------------------------------------------------------

/// Check that hosts with multiple `IdentityFile` entries also have
/// `IdentitiesOnly yes` to prevent the agent from offering wrong keys.
struct IdentitiesOnlyCheck<'a> {
    paths: &'a SshPaths,
}

impl Check for IdentitiesOnlyCheck<'_> {
    fn id(&self) -> &'static str {
        "identities_only"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(&self) -> CheckFuture<'_> {
        let config_path = self.paths.config_path().to_path_buf();
        Box::pin(async move {
            let Ok(content) = tokio::fs::read_to_string(&config_path).await else {
                return Ok(vec![]);
            };

            let ast = ast::parse(&content);
            let mut diagnostics = Vec::new();

            for node in &ast.nodes {
                if let ConfigNode::HostBlock { patterns, nodes, .. } = node {
                    let id_count = nodes
                        .iter()
                        .filter(|n| {
                            matches!(n, ConfigNode::Directive { keyword, .. }
                                if keyword.eq_ignore_ascii_case("IdentityFile"))
                        })
                        .count();

                    if id_count > 1 {
                        let has_identities_only = nodes.iter().any(|n| {
                            matches!(n, ConfigNode::Directive { keyword, value, .. }
                                if keyword.eq_ignore_ascii_case("IdentitiesOnly")
                                    && value.eq_ignore_ascii_case("yes"))
                        });

                        if !has_identities_only {
                            let host_str = patterns.join(", ");
                            diagnostics.push(Diagnostic {
                                id: "identities_only",
                                severity: Severity::Warning,
                                message: format!(
                                    "Host {host_str} has {id_count} IdentityFile entries \
                                     but no IdentitiesOnly yes"
                                ),
                                hint: Some(
                                    "Add IdentitiesOnly yes to prevent the agent from \
                                     offering keys not listed in this Host block"
                                        .into(),
                                ),
                                module: "local",
                            });
                        }
                    }
                }
            }

            if diagnostics.is_empty() {
                Ok(vec![Diagnostic {
                    id: "identities_only",
                    severity: Severity::Ok,
                    message: "No multi-key Host blocks missing IdentitiesOnly".into(),
                    hint: None,
                    module: "local",
                }])
            } else {
                Ok(diagnostics)
            }
        })
    }
}

// ---------------------------------------------------------------------------
// HomeDirPermissionsCheck — StrictModes full chain
// ---------------------------------------------------------------------------

/// Check that the home directory is not group/world writable (StrictModes).
#[cfg(unix)]
struct HomeDirPermissionsCheck;

#[cfg(unix)]
impl Check for HomeDirPermissionsCheck {
    fn id(&self) -> &'static str {
        "home_dir_permissions"
    }
    fn module(&self) -> &'static str {
        "local"
    }
    fn run(&self) -> CheckFuture<'_> {
        Box::pin(async move {
            let Some(home) = dirs::home_dir() else {
                return Ok(vec![Diagnostic {
                    id: "home_dir_permissions",
                    severity: Severity::Info,
                    message: "Cannot determine home directory".into(),
                    hint: None,
                    module: "local",
                }]);
            };

            let Ok(meta) = tokio::fs::metadata(&home).await else {
                return Ok(vec![Diagnostic {
                    id: "home_dir_permissions",
                    severity: Severity::Warning,
                    message: format!("Cannot read home directory: {}", home.display()),
                    hint: None,
                    module: "local",
                }]);
            };

            let mode = meta.permissions().mode();
            let group_write = mode & 0o020 != 0;
            let other_write = mode & 0o002 != 0;

            if group_write || other_write {
                let who = match (group_write, other_write) {
                    (true, true) => "group and world",
                    (true, false) => "group",
                    (false, true) => "world",
                    _ => unreachable!(),
                };
                Ok(vec![Diagnostic {
                    id: "home_dir_permissions",
                    severity: Severity::Warning,
                    message: format!(
                        "Home directory {} is {} writable — \
                         sshd StrictModes may reject authorized_keys",
                        home.display(),
                        who,
                    ),
                    hint: Some(format!(
                        "chmod g-w,o-w {}",
                        home.display(),
                    )),
                    module: "local",
                }])
            } else {
                Ok(vec![Diagnostic {
                    id: "home_dir_permissions",
                    severity: Severity::Ok,
                    message: format!(
                        "Home directory {} has correct permissions",
                        home.display()
                    ),
                    hint: None,
                    module: "local",
                }])
            }
        })
    }
}

// ---------------------------------------------------------------------------
// run_all — execute every local check
// ---------------------------------------------------------------------------

/// Run all local diagnostic checks.
pub async fn run_all<'a>(paths: &'a SshPaths) -> Result<Vec<Diagnostic>> {
    let mut checks: Vec<Box<dyn Check + 'a>> = vec![
        Box::new(SshDirExists { paths }),
    ];

    checks.push(Box::new(SshDirPermissions { paths }));

    checks.push(Box::new(ConfigExists { paths }));
    checks.push(Box::new(KnownHostsExists { paths }));

    checks.push(Box::new(PrivateKeyPermissions { paths }));

    checks.push(Box::new(OwnerCheck { paths }));

    checks.push(Box::new(AgentAvailable));
    checks.push(Box::new(KeygenAvailable));
    checks.push(Box::new(DefaultKeyExists { paths }));

    checks.push(Box::new(ConfigPermissionsCheck { paths }));

    checks.push(Box::new(AuthorizedKeysPermissionsCheck { paths }));

    checks.push(Box::new(PublicKeyPairsCheck { paths }));
    checks.push(Box::new(IdentityFileExistsCheck { paths }));
    checks.push(Box::new(DuplicateHostCheck { paths }));
    checks.push(Box::new(HostStarPlacementCheck { paths }));
    checks.push(Box::new(SshV1KeyCheck { paths }));
    checks.push(Box::new(IdentityFilePubCheck { paths }));
    checks.push(Box::new(IdentitiesOnlyCheck { paths }));
    #[cfg(unix)]
    checks.push(Box::new(HomeDirPermissionsCheck));
    checks.push(Box::new(PlatformCheck));

    let mut all_diagnostics = Vec::new();
    for check in &checks {
        match check.run().await {
            Ok(diagnostics) => all_diagnostics.extend(diagnostics),
            Err(e) => all_diagnostics.push(Diagnostic {
                id: check.id(),
                severity: Severity::Error,
                message: format!("Check failed: {e}"),
                hint: None,
                module: check.module(),
            }),
        }
    }
    Ok(all_diagnostics)
}

#[cfg(test)]
#[path = "local.test.rs"]
mod tests;
