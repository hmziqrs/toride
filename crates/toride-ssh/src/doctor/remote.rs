//! Remote SSH diagnostic checks.

use crate::paths::SshPaths;
use crate::types::{Diagnostic, Severity};
use crate::{Error, Result};

/// Check whether the remote host is reachable via SSH.
struct HostReachable<'a> {
    host: &'a str,
}

/// Check whether the host key is present in known_hosts.
struct HostKeyKnown<'a> {
    paths: &'a SshPaths,
    host: &'a str,
}

/// Check whether agent forwarding works to the remote host.
struct AgentForwarding<'a> {
    host: &'a str,
}

/// Check that remote `~/.ssh` has restrictive permissions (should be 700).
struct RemotePermissionsCheck<'a> {
    host: &'a str,
}

/// Check that remote `~/.ssh/authorized_keys` exists.
struct RemoteAuthorizedKeysCheck<'a> {
    host: &'a str,
}

/// Verify that public-key authentication works to the remote host.
struct RemotePubkeyAuthCheck<'a> {
    host: &'a str,
}

/// Check remote sshd configuration for common misconfigurations.
struct RemoteSshdConfigCheck<'a> {
    host: &'a str,
}

/// Verify the remote home directory is accessible and exists.
struct RemoteHomeCheck<'a> {
    host: &'a str,
}

// ---------------------------------------------------------------------------
// Check implementations
// ---------------------------------------------------------------------------

impl HostReachable<'_> {
    async fn run_check(&self) -> Result<Vec<Diagnostic>> {
        // Verify `ssh` is available before attempting to connect.
        if !crate::runner::tool_exists("ssh") {
            return Ok(vec![Diagnostic {
                id: "host_reachable",
                severity: Severity::Error,
                message: "`ssh` is not found in PATH".into(),
                hint: Some(
                    "Install OpenSSH: `brew install openssh` (macOS) or `sudo apt install openssh-client` (Linux)".into(),
                ),
                module: "remote",
            }]);
        }

        let output = tokio::process::Command::new("ssh")
            .args([
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=5",
                "-o",
                "StrictHostKeyChecking=accept-new",
                self.host,
                "true",
            ])
            .output()
            .await
            .map_err(|e| Error::CommandFailed(format!("failed to execute ssh: {e}")))?;

        if output.status.success() {
            Ok(vec![Diagnostic {
                id: "host_reachable",
                severity: Severity::Ok,
                message: format!("Successfully connected to {}", self.host),
                hint: None,
                module: "remote",
            }])
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(vec![Diagnostic {
                id: "host_reachable",
                severity: Severity::Error,
                message: format!("Cannot connect to {}: {}", self.host, stderr.trim()),
                hint: Some(
                    "Verify the hostname, network connectivity, and that SSH is running on the remote".into(),
                ),
                module: "remote",
            }])
        }
    }
}

impl HostKeyKnown<'_> {
    async fn run_check(&self) -> Result<Vec<Diagnostic>> {
        let kh_path = self.paths.known_hosts_path();

        match tokio::fs::read_to_string(&kh_path).await {
            Ok(content) => {
                // Check if the host appears in a known_hosts line. This is a
                // best-effort check: hashed known_hosts entries (HashKnownHosts yes)
                // cannot be matched by name and will be reported as unknown.
                let bracketed = format!("[{}]", self.host);
                let known = content.lines().any(|line| {
                    if line.is_empty() || line.starts_with('#') || line.starts_with('@') {
                        return false;
                    }
                    // The host field is the first token (comma-separated for
                    // multiple hosts, with optional patterns like *.example.com).
                    let Some(host_field) = line.split_whitespace().next() else {
                        return false;
                    };
                    // Skip hashed entries — we cannot verify them by name.
                    if host_field.starts_with('|') {
                        return false;
                    }
                    host_field
                        .split(',')
                        .any(|h| h == self.host || h == bracketed)
                });

                if known {
                    Ok(vec![Diagnostic {
                        id: "host_key_known",
                        severity: Severity::Ok,
                        message: format!("{} is present in known_hosts", self.host),
                        hint: None,
                        module: "remote",
                    }])
                } else {
                    Ok(vec![Diagnostic {
                        id: "host_key_known",
                        severity: Severity::Warning,
                        message: format!(
                            "{} is not in known_hosts — you will be prompted on first connection",
                            self.host
                        ),
                        hint: Some(format!(
                            "Run `ssh-keyscan -H {} >> ~/.ssh/known_hosts` to add it",
                            self.host
                        )),
                        module: "remote",
                    }])
                }
            }
            Err(_) => Ok(vec![Diagnostic {
                id: "host_key_known",
                severity: Severity::Warning,
                message: "known_hosts file does not exist".into(),
                hint: Some("Run `touch ~/.ssh/known_hosts`".into()),
                module: "remote",
            }]),
        }
    }
}

impl AgentForwarding<'_> {
    async fn run_check(&self) -> Result<Vec<Diagnostic>> {
        // The `-A` flag enables agent forwarding. This is safe for diagnostic
        // purposes (a brief `ssh-add -l` invocation) but note that agent
        // forwarding should not be used on untrusted remote hosts in general
        // because a compromised remote could use the forwarded agent socket.

        if std::env::var("SSH_AUTH_SOCK").map_or(true, |v| v.is_empty()) {
            return Ok(vec![Diagnostic {
                id: "agent_forwarding",
                severity: Severity::Warning,
                message: "SSH agent is not running locally — cannot test agent forwarding".into(),
                hint: Some("Start the SSH agent: `eval $(ssh-agent -s)`".into()),
                module: "remote",
            }]);
        }

        let output = tokio::process::Command::new("ssh")
            .args([
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=5",
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-A",
                self.host,
                "ssh-add -l",
            ])
            .output()
            .await
            .map_err(|e| Error::CommandFailed(format!("failed to execute ssh: {e}")))?;

        if output.status.success() {
            Ok(vec![Diagnostic {
                id: "agent_forwarding",
                severity: Severity::Ok,
                message: format!("Agent forwarding works to {}", self.host),
                hint: None,
                module: "remote",
            }])
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(vec![Diagnostic {
                id: "agent_forwarding",
                severity: Severity::Warning,
                message: format!(
                    "Agent forwarding may not work to {}: {}",
                    self.host,
                    stderr.trim()
                ),
                hint: Some(
                    "Ensure `AllowAgentForwarding yes` is set in the remote sshd_config".into(),
                ),
                module: "remote",
            }])
        }
    }
}

// ---------------------------------------------------------------------------
// RemotePermissionsCheck
// ---------------------------------------------------------------------------

impl RemotePermissionsCheck<'_> {
    async fn run_check(&self) -> Result<Vec<Diagnostic>> {
        let output = tokio::process::Command::new("ssh")
            .args([
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=5",
                "-o",
                "StrictHostKeyChecking=accept-new",
                self.host,
                // Use `stat -c %a` for GNU/Linux and fall back to `stat -f %Lp` for BSD/macOS.
                // The portable idiom `$(stat -c %a . 2>/dev/null || stat -f %Lp .)` covers both.
                "stat -c '%a' ~/.ssh 2>/dev/null || stat -f '%Lp' ~/.ssh 2>/dev/null",
            ])
            .output()
            .await
            .map_err(|e| Error::CommandFailed(format!("failed to execute ssh: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(vec![Diagnostic {
                id: "remote_permissions",
                severity: Severity::Warning,
                message: format!(
                    "Could not stat remote ~/.ssh on {}: {}",
                    self.host,
                    stderr.trim()
                ),
                hint: Some("Ensure ~/.ssh exists on the remote host".into()),
                module: "remote",
            }]);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mode = stdout.trim().trim_matches('\'');

        if mode == "700" {
            Ok(vec![Diagnostic {
                id: "remote_permissions",
                severity: Severity::Ok,
                message: format!("Remote ~/.ssh permissions are {mode} (correct)"),
                hint: None,
                module: "remote",
            }])
        } else {
            Ok(vec![Diagnostic {
                id: "remote_permissions",
                severity: Severity::Warning,
                message: format!(
                    "Remote ~/.ssh permissions are {mode} on {} (should be 700)",
                    self.host
                ),
                hint: Some("Run on the remote: `chmod 700 ~/.ssh`".into()),
                module: "remote",
            }])
        }
    }
}

// ---------------------------------------------------------------------------
// RemoteAuthorizedKeysCheck
// ---------------------------------------------------------------------------

impl RemoteAuthorizedKeysCheck<'_> {
    async fn run_check(&self) -> Result<Vec<Diagnostic>> {
        let output = tokio::process::Command::new("ssh")
            .args([
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=5",
                "-o",
                "StrictHostKeyChecking=accept-new",
                self.host,
                "test -f ~/.ssh/authorized_keys",
            ])
            .output()
            .await
            .map_err(|e| Error::CommandFailed(format!("failed to execute ssh: {e}")))?;

        if output.status.success() {
            Ok(vec![Diagnostic {
                id: "remote_authorized_keys",
                severity: Severity::Ok,
                message: format!("~/.ssh/authorized_keys exists on {}", self.host),
                hint: None,
                module: "remote",
            }])
        } else {
            Ok(vec![Diagnostic {
                id: "remote_authorized_keys",
                severity: Severity::Warning,
                message: format!(
                    "~/.ssh/authorized_keys does not exist on {}",
                    self.host
                ),
                hint: Some(
                    "Copy your public key: `ssh-copy-id <user>@<host>` or manually create the file".into(),
                ),
                module: "remote",
            }])
        }
    }
}

// ---------------------------------------------------------------------------
// RemotePubkeyAuthCheck
// ---------------------------------------------------------------------------

impl RemotePubkeyAuthCheck<'_> {
    async fn run_check(&self) -> Result<Vec<Diagnostic>> {
        let output = tokio::process::Command::new("ssh")
            .args([
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=5",
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-o",
                "PreferredAuthentications=publickey",
                self.host,
                "true",
            ])
            .output()
            .await
            .map_err(|e| Error::CommandFailed(format!("failed to execute ssh: {e}")))?;

        if output.status.success() {
            Ok(vec![Diagnostic {
                id: "remote_pubkey_auth",
                severity: Severity::Ok,
                message: format!("Public-key authentication works to {}", self.host),
                hint: None,
                module: "remote",
            }])
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(vec![Diagnostic {
                id: "remote_pubkey_auth",
                severity: Severity::Error,
                message: format!(
                    "Public-key authentication failed to {}: {}",
                    self.host,
                    stderr.trim()
                ),
                hint: Some(
                    "Ensure your public key is in the remote ~/.ssh/authorized_keys and the key is loaded in the agent".into(),
                ),
                module: "remote",
            }])
        }
    }
}

// ---------------------------------------------------------------------------
// RemoteSshdConfigCheck
// ---------------------------------------------------------------------------

impl RemoteSshdConfigCheck<'_> {
    #[allow(clippy::too_many_lines)]
    async fn run_check(&self) -> Result<Vec<Diagnostic>> {
        let output = tokio::process::Command::new("ssh")
            .args([
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=5",
                "-o",
                "StrictHostKeyChecking=accept-new",
                self.host,
                "sshd -T 2>/dev/null",
            ])
            .output()
            .await
            .map_err(|e| Error::CommandFailed(format!("failed to execute ssh: {e}")))?;

        if !output.status.success() {
            return Ok(vec![Diagnostic {
                id: "remote_sshd_config",
                severity: Severity::Info,
                message: format!(
                    "Could not read sshd config on {} (may require elevated privileges)",
                    self.host
                ),
                hint: Some(
                    "Ensure `PubkeyAuthentication yes` and `AllowAgentForwarding yes` are set in /etc/ssh/sshd_config".into(),
                ),
                module: "remote",
            }]);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut diagnostics = Vec::new();

        let pubkey_auth = find_sshd_setting(&stdout, "pubkeyauthentication");
        match pubkey_auth.as_deref() {
            Some("yes") => {
                diagnostics.push(Diagnostic {
                    id: "remote_sshd_config",
                    severity: Severity::Ok,
                    message: format!("sshd PubkeyAuthentication is enabled on {}", self.host),
                    hint: None,
                    module: "remote",
                });
            }
            Some(other) => {
                diagnostics.push(Diagnostic {
                    id: "remote_sshd_config",
                    severity: Severity::Warning,
                    message: format!(
                        "sshd PubkeyAuthentication is '{}' on {} (expected 'yes')",
                        other, self.host
                    ),
                    hint: Some("Set `PubkeyAuthentication yes` in /etc/ssh/sshd_config".into()),
                    module: "remote",
                });
            }
            None => {
                diagnostics.push(Diagnostic {
                    id: "remote_sshd_config",
                    severity: Severity::Ok,
                    message: format!(
                        "sshd PubkeyAuthentication not explicitly set on {} (defaults to yes)",
                        self.host
                    ),
                    hint: None,
                    module: "remote",
                });
            }
        }

        let agent_fwd = find_sshd_setting(&stdout, "allowagentforwarding");
        match agent_fwd.as_deref() {
            Some("yes") => {
                diagnostics.push(Diagnostic {
                    id: "remote_sshd_config",
                    severity: Severity::Ok,
                    message: format!("sshd AllowAgentForwarding is enabled on {}", self.host),
                    hint: None,
                    module: "remote",
                });
            }
            Some(other) => {
                diagnostics.push(Diagnostic {
                    id: "remote_sshd_config",
                    severity: Severity::Warning,
                    message: format!(
                        "sshd AllowAgentForwarding is '{}' on {} (expected 'yes')",
                        other, self.host
                    ),
                    hint: Some(
                        "Set `AllowAgentForwarding yes` in /etc/ssh/sshd_config".into(),
                    ),
                    module: "remote",
                });
            }
            None => {
                diagnostics.push(Diagnostic {
                    id: "remote_sshd_config",
                    severity: Severity::Ok,
                    message: format!(
                        "sshd AllowAgentForwarding not explicitly set on {} (defaults to yes)",
                        self.host
                    ),
                    hint: None,
                    module: "remote",
                });
            }
        }

        Ok(diagnostics)
    }
}

/// Find a setting value from `sshd -T` output (lowercased key-value pairs).
fn find_sshd_setting(config: &str, key: &str) -> Option<String> {
    for line in config.lines() {
        if let Some(rest) = line.strip_prefix(key)
            && (rest.starts_with(' ') || rest.starts_with('\t'))
        {
            return Some(rest.trim().to_lowercase());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// RemoteHomeCheck
// ---------------------------------------------------------------------------

impl RemoteHomeCheck<'_> {
    async fn run_check(&self) -> Result<Vec<Diagnostic>> {
        let output = tokio::process::Command::new("ssh")
            .args([
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=5",
                "-o",
                "StrictHostKeyChecking=accept-new",
                self.host,
                "echo $HOME",
            ])
            .output()
            .await
            .map_err(|e| Error::CommandFailed(format!("failed to execute ssh: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(vec![Diagnostic {
                id: "remote_home",
                severity: Severity::Error,
                message: format!(
                    "Could not determine remote home directory on {}: {}",
                    self.host,
                    stderr.trim()
                ),
                hint: Some("Verify the remote host is accessible and the shell is configured".into()),
                module: "remote",
            }]);
        }

        let home = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if home.is_empty() {
            Ok(vec![Diagnostic {
                id: "remote_home",
                severity: Severity::Warning,
                message: format!("Remote $HOME is empty on {}", self.host),
                hint: Some("Ensure the remote user has a properly configured shell environment".into()),
                module: "remote",
            }])
        } else {
            Ok(vec![Diagnostic {
                id: "remote_home",
                severity: Severity::Ok,
                message: format!("Remote home directory is '{}' on {}", home, self.host),
                hint: None,
                module: "remote",
            }])
        }
    }
}

// ---------------------------------------------------------------------------
// run_all — execute every remote check
// ---------------------------------------------------------------------------

/// Run all remote diagnostic checks for a host.
///
/// If the host is unreachable, subsequent checks (known_hosts, agent forwarding)
/// are still run — the known_hosts check is local, and agent forwarding provides
/// a useful diagnostic regardless. However, if the host-reachability check itself
/// errors (not just returns a diagnostic), we short-circuit to avoid cascading
/// failures.
pub async fn run_all(paths: &SshPaths, host: &str) -> Result<Vec<Diagnostic>> {
    let mut all_diagnostics = Vec::new();

    // Host reachability
    let check = HostReachable { host };
    let host_reachable = match check.run_check().await {
        Ok(d) => {
            let was_ok = d.iter().any(|d| d.severity == Severity::Ok);
            all_diagnostics.extend(d);
            was_ok
        }
        Err(e) => {
            all_diagnostics.push(err_diagnostic("host_reachable", &e));
            false
        }
    };

    // Host key in known_hosts (local check, always runs).
    let check = HostKeyKnown { paths, host };
    match check.run_check().await {
        Ok(d) => all_diagnostics.extend(d),
        Err(e) => all_diagnostics.push(err_diagnostic("host_key_known", &e)),
    }

    // Agent forwarding — skip if host is not reachable to avoid a slow timeout.
    if host_reachable {
        let check = AgentForwarding { host };
        match check.run_check().await {
            Ok(d) => all_diagnostics.extend(d),
            Err(e) => all_diagnostics.push(err_diagnostic("agent_forwarding", &e)),
        }

        // Remote home directory
        let check = RemoteHomeCheck { host };
        match check.run_check().await {
            Ok(d) => all_diagnostics.extend(d),
            Err(e) => all_diagnostics.push(err_diagnostic("remote_home", &e)),
        }

        // Public-key authentication
        let check = RemotePubkeyAuthCheck { host };
        match check.run_check().await {
            Ok(d) => all_diagnostics.extend(d),
            Err(e) => all_diagnostics.push(err_diagnostic("remote_pubkey_auth", &e)),
        }

        // Remote ~/.ssh permissions
        let check = RemotePermissionsCheck { host };
        match check.run_check().await {
            Ok(d) => all_diagnostics.extend(d),
            Err(e) => all_diagnostics.push(err_diagnostic("remote_permissions", &e)),
        }

        // Remote authorized_keys existence
        let check = RemoteAuthorizedKeysCheck { host };
        match check.run_check().await {
            Ok(d) => all_diagnostics.extend(d),
            Err(e) => all_diagnostics.push(err_diagnostic("remote_authorized_keys", &e)),
        }

        // Remote sshd configuration
        let check = RemoteSshdConfigCheck { host };
        match check.run_check().await {
            Ok(d) => all_diagnostics.extend(d),
            Err(e) => all_diagnostics.push(err_diagnostic("remote_sshd_config", &e)),
        }
    } else {
        let skipped = [
            "agent_forwarding",
            "remote_home",
            "remote_pubkey_auth",
            "remote_permissions",
            "remote_authorized_keys",
            "remote_sshd_config",
        ];
        for id in skipped {
            all_diagnostics.push(Diagnostic {
                id,
                severity: Severity::Info,
                message: format!("Skipped {id} check: host is not reachable"),
                hint: None,
                module: "remote",
            });
        }
    }

    Ok(all_diagnostics)
}

fn err_diagnostic(id: &'static str, e: &Error) -> Diagnostic {
    Diagnostic {
        id,
        severity: Severity::Error,
        message: format!("Remote check '{id}' failed: {e}"),
        hint: None,
        module: "remote",
    }
}
