//! Async SSH data collection.
//!
//! [`SshDataCollector`] manages background collection of all SSH subsystem data
//! via a tokio oneshot channel, following the same pattern as [`StatusCollector`].
//!
//! Currently seeds mock data. Will be wired to [`SshManager`] in a later phase.

use tokio::sync::oneshot;

use crate::ui::screens::ssh::{
    AgentKeyEntry, AgentStatus, AuthorizedKeyEntry, CertificateEntry, ConfigHostEntry,
    DiagnosticEntry, ForwardEntry, ForwardSessionEntry, KnownHostEntry, SshKeyEntry,
};

/// Aggregated SSH data for all tabs.
pub struct SshDataBundle {
    /// SSH key entries.
    pub keys: Vec<SshKeyEntry>,
    /// Known hosts entries.
    pub known_hosts: Vec<KnownHostEntry>,
    /// SSH config host blocks.
    pub config_hosts: Vec<ConfigHostEntry>,
    /// SSH agent connection status.
    pub agent_status: AgentStatus,
    /// Keys loaded in the SSH agent.
    pub agent_keys: Vec<AgentKeyEntry>,
    /// Active port forwarding sessions.
    pub forwarding: Vec<ForwardSessionEntry>,
    /// Diagnostic check results.
    pub diagnostics: Vec<DiagnosticEntry>,
    /// Authorized keys entries.
    pub authorized_keys: Vec<AuthorizedKeyEntry>,
    /// SSH certificate entries.
    pub certificates: Vec<CertificateEntry>,
}

/// Manages periodic async collection of SSH data.
pub struct SshDataCollector {
    rx: Option<oneshot::Receiver<SshDataBundle>>,
}

impl SshDataCollector {
    /// Create a new collector with no pending collection.
    #[must_use]
    pub fn new() -> Self {
        Self { rx: None }
    }

    /// Whether a collection is currently in-flight.
    pub fn is_pending(&self) -> bool {
        self.rx.is_some()
    }

    /// Start a new background collection.
    ///
    /// If a collection is already in-flight, this is a no-op.
    pub fn start(&mut self) {
        if self.rx.is_some() {
            return;
        }
        let (tx, rx) = oneshot::channel();
        self.rx = Some(rx);
        tokio::spawn(async move {
            let bundle = collect_mock_data();
            let _ = tx.send(bundle);
        });
    }

    /// Poll for a completed collection result.
    ///
    /// Returns `Some(bundle)` if the collection completed, `None` if still
    /// pending or if the collection failed.
    pub async fn poll(&mut self) -> Option<SshDataBundle> {
        match &mut self.rx {
            Some(rx) => {
                let result = rx.await.ok();
                self.rx = None;
                result
            }
            None => None,
        }
    }
}

impl Default for SshDataCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Collect mock SSH data for all subsystems.
///
/// TODO: Replace with `SshManager` calls in Phase 2.
fn collect_mock_data() -> SshDataBundle {
    SshDataBundle {
        keys: collect_mock_keys(),
        known_hosts: collect_mock_known_hosts(),
        config_hosts: collect_mock_config_hosts(),
        agent_status: collect_mock_agent_status(),
        agent_keys: collect_mock_agent_keys(),
        forwarding: collect_mock_forwarding(),
        diagnostics: collect_mock_diagnostics(),
        authorized_keys: collect_mock_authorized_keys(),
        certificates: collect_mock_certificates(),
    }
}

fn collect_mock_keys() -> Vec<SshKeyEntry> {
    vec![
        SshKeyEntry {
            name: "id_ed25519".into(),
            key_type: "Ed25519".into(),
            fingerprint: "SHA256:abc123def456ghi789".into(),
            encrypted: true,
            permissions: "0600".into(),
            has_public: true,
            has_cert: false,
            host_count: 2,
        },
        SshKeyEntry {
            name: "id_rsa".into(),
            key_type: "RSA 4096".into(),
            fingerprint: "SHA256:xyz789abc456def123".into(),
            encrypted: false,
            permissions: "0644".into(),
            has_public: true,
            has_cert: true,
            host_count: 0,
        },
        SshKeyEntry {
            name: "deploy_key".into(),
            key_type: "Ed25519".into(),
            fingerprint: "SHA256:qwe456rty789uio012".into(),
            encrypted: false,
            permissions: "0600".into(),
            has_public: true,
            has_cert: false,
            host_count: 5,
        },
    ]
}

fn collect_mock_known_hosts() -> Vec<KnownHostEntry> {
    vec![
        KnownHostEntry {
            host: "github.com".into(),
            key_type: "ssh-ed25519".into(),
            fingerprint: "SHA256:nThbg6kXUpJWGl7E1IGOCspRomTxdCARLviKw6E5SY8".into(),
            is_hashed: false,
            marker: None,
            has_comment: false,
        },
        KnownHostEntry {
            host: "gitlab.com".into(),
            key_type: "ssh-ed25519".into(),
            fingerprint: "SHA256:WSCtr3bEeJGgcb0UrkMFWxQJqchWXzwWMNESdgqxo".into(),
            is_hashed: false,
            marker: None,
            has_comment: false,
        },
        KnownHostEntry {
            host: "[192.168.1.1]:2222".into(),
            key_type: "ssh-rsa".into(),
            fingerprint: "SHA256:abc123def456ghi789jkl012mno345pqr678".into(),
            is_hashed: false,
            marker: None,
            has_comment: true,
        },
        KnownHostEntry {
            host: "|1|ba4dEeFgHiJkLmNoPqRsTu|XxYyZz0123456789".into(),
            key_type: "ecdsa-sha2-nistp256".into(),
            fingerprint: "SHA256:qwe456rty789uio012pqr345stu678vwx".into(),
            is_hashed: true,
            marker: None,
            has_comment: false,
        },
        KnownHostEntry {
            host: "old.server.example.com".into(),
            key_type: "ssh-ed25519".into(),
            fingerprint: "SHA256:xyz789abc456def123ghi456jkl789mno012".into(),
            is_hashed: false,
            marker: Some("@revoked".into()),
            has_comment: false,
        },
    ]
}

fn collect_mock_config_hosts() -> Vec<ConfigHostEntry> {
    vec![
        ConfigHostEntry {
            name: "myserver".into(),
            patterns: vec!["myserver".into()],
            host_name: Some("example.com".into()),
            user: Some("alice".into()),
            port: Some(2222),
            identity_file: Some("~/.ssh/id_ed25519".into()),
            proxy_jump: None,
            directive_count: 5,
            has_diagnostic: false,
        },
        ConfigHostEntry {
            name: "*.example.com".into(),
            patterns: vec!["*.example.com".into()],
            host_name: None,
            user: Some("deploy".into()),
            port: None,
            identity_file: None,
            proxy_jump: None,
            directive_count: 3,
            has_diagnostic: false,
        },
        ConfigHostEntry {
            name: "*".into(),
            patterns: vec!["*".into()],
            host_name: None,
            user: None,
            port: None,
            identity_file: None,
            proxy_jump: None,
            directive_count: 2,
            has_diagnostic: false,
        },
        ConfigHostEntry {
            name: "staging".into(),
            patterns: vec!["staging".into()],
            host_name: Some("stage.example.com".into()),
            user: Some("bob".into()),
            port: Some(22),
            identity_file: Some("~/.ssh/deploy_key".into()),
            proxy_jump: Some("bastion.example.com".into()),
            directive_count: 8,
            has_diagnostic: true,
        },
        ConfigHostEntry {
            name: "bastion".into(),
            patterns: vec!["bastion.example.com".into()],
            host_name: None,
            user: Some("admin".into()),
            port: Some(443),
            identity_file: Some("~/.ssh/id_ed25519".into()),
            proxy_jump: None,
            directive_count: 4,
            has_diagnostic: false,
        },
    ]
}

fn collect_mock_agent_status() -> AgentStatus {
    AgentStatus {
        reachable: true,
        socket_path: Some("/tmp/ssh-abc123/agent.1234".into()),
        key_count: 3,
    }
}

fn collect_mock_agent_keys() -> Vec<AgentKeyEntry> {
    vec![
        AgentKeyEntry {
            name: "id_ed25519".into(),
            key_type: "Ed25519".into(),
            fingerprint: "SHA256:abc123def456ghi789".into(),
            is_locked: false,
            has_constraints: false,
        },
        AgentKeyEntry {
            name: "deploy_key".into(),
            key_type: "RSA 4096".into(),
            fingerprint: "SHA256:xyz789abc456def123".into(),
            is_locked: true,
            has_constraints: true,
        },
        AgentKeyEntry {
            name: "staging_key".into(),
            key_type: "Ed25519".into(),
            fingerprint: "SHA256:qwe456rty789uio012".into(),
            is_locked: false,
            has_constraints: false,
        },
    ]
}

fn collect_mock_forwarding() -> Vec<ForwardSessionEntry> {
    vec![
        ForwardSessionEntry {
            host: "myserver".into(),
            control_path: "/home/alice/.ssh/cm-alice@example.com:22".into(),
            pid: Some(1234),
            established_ago: "2h 15m".into(),
            forward_count: 2,
            forwards: vec![
                ForwardEntry {
                    forward_type: "local".into(),
                    local_addr: "127.0.0.1".into(),
                    local_port: 8080,
                    remote_addr: "example.com".into(),
                    remote_port: 80,
                },
                ForwardEntry {
                    forward_type: "local".into(),
                    local_addr: "127.0.0.1".into(),
                    local_port: 3306,
                    remote_addr: "db.example.com".into(),
                    remote_port: 3306,
                },
            ],
        },
        ForwardSessionEntry {
            host: "bastion".into(),
            control_path: "/home/alice/.ssh/ctrl-bastion".into(),
            pid: Some(5678),
            established_ago: "45m".into(),
            forward_count: 2,
            forwards: vec![
                ForwardEntry {
                    forward_type: "dynamic".into(),
                    local_addr: "127.0.0.1".into(),
                    local_port: 1080,
                    remote_addr: "SOCKS".into(),
                    remote_port: 0,
                },
                ForwardEntry {
                    forward_type: "remote".into(),
                    local_addr: "0.0.0.0".into(),
                    local_port: 2222,
                    remote_addr: "127.0.0.1".into(),
                    remote_port: 22,
                },
            ],
        },
    ]
}

fn collect_mock_diagnostics() -> Vec<DiagnosticEntry> {
    vec![
        DiagnosticEntry {
            id: "ssh_dir_exists".into(),
            severity: "ok".into(),
            module: "local".into(),
            message: "SSH directory exists with correct permissions (0700)".into(),
            hint: None,
        },
        DiagnosticEntry {
            id: "config_found".into(),
            severity: "info".into(),
            module: "config".into(),
            message: "SSH config file found at ~/.ssh/config".into(),
            hint: None,
        },
        DiagnosticEntry {
            id: "key_permissions".into(),
            severity: "warning".into(),
            module: "local".into(),
            message: "Private key id_rsa has overly permissive mode (0644)".into(),
            hint: Some("Run chmod 600 ~/.ssh/id_rsa to fix".into()),
        },
        DiagnosticEntry {
            id: "agent_not_running".into(),
            severity: "error".into(),
            module: "agent".into(),
            message: "No SSH agent is running (SSH_AUTH_SOCK not set)".into(),
            hint: Some("Start ssh-agent or add eval $(ssh-agent) to your shell profile".into()),
        },
        DiagnosticEntry {
            id: "config_host_star_placement".into(),
            severity: "warning".into(),
            module: "config".into(),
            message: "'Host *' appears before specific Host blocks".into(),
            hint: Some("Move 'Host *' to the end of the config file".into()),
        },
        DiagnosticEntry {
            id: "known_hosts_exists".into(),
            severity: "ok".into(),
            module: "local".into(),
            message: "Known hosts file exists at ~/.ssh/known_hosts".into(),
            hint: None,
        },
    ]
}

fn collect_mock_authorized_keys() -> Vec<AuthorizedKeyEntry> {
    vec![
        AuthorizedKeyEntry {
            key_type: "ssh-ed25519".into(),
            public_key: "AAAAC3NzaC1lZDI1NTE5AAAAIKxJ3G2F7mT5mQaV8eN4pL2zH8gR6kW".into(),
            comment: Some("alice@workstation".into()),
            fingerprint: "SHA256:xKj8mN2pL5vR7tQ9wE3yU4oI6aS8dF".into(),
            options: None,
            line: 1,
        },
        AuthorizedKeyEntry {
            key_type: "ssh-rsa".into(),
            public_key: "AAAAB3NzaC1yc2EAAAADAQABAAACAQCr7L3hFS2jW9eJ5kE8mN".into(),
            comment: Some("deploy@ci-runner".into()),
            fingerprint: "SHA256:mQ9wE3yU4oI6aS8dFxKj8mN2pL5vR7t".into(),
            options: Some("command=\"/usr/bin/restricted-shell\",no-port-forwarding".into()),
            line: 4,
        },
        AuthorizedKeyEntry {
            key_type: "ssh-ed25519".into(),
            public_key: "AAAAC3NzaC1lZDI1NTE5AAAAIP9fG4eJ8kL3mN6oQ2rS5tU7vW".into(),
            comment: Some("bob@laptop".into()),
            fingerprint: "SHA256:R7tQ9wE3yU4oI6aS8dFxKj8mN2pL5v".into(),
            options: None,
            line: 7,
        },
        AuthorizedKeyEntry {
            key_type: "ecdsa-sha2-nistp256".into(),
            public_key: "AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTY".into(),
            comment: None,
            fingerprint: "SHA256:U4oI6aS8dFxKj8mN2pL5vR7tQ9wE3y".into(),
            options: Some("no-pty".into()),
            line: 9,
        },
    ]
}

fn collect_mock_certificates() -> Vec<CertificateEntry> {
    vec![
        CertificateEntry {
            name: "id_ed25519-cert.pub".into(),
            cert_type: "User".into(),
            key_type: "ssh-ed25519-cert-v01@openssh.com".into(),
            serial: 12345,
            valid_from: "2025-01-15 00:00:00".into(),
            valid_to: "2026-01-15 00:00:00".into(),
            is_valid: true,
            ca_fingerprint: "SHA256:CA1fP2gH3iJ4kL5mN6oQ7rS8tU".into(),
            key_id: "alice@corp-2025".into(),
            principals: vec!["alice".into(), "admin".into()],
        },
        CertificateEntry {
            name: "deploy-cert.pub".into(),
            cert_type: "User".into(),
            key_type: "ssh-ed25519-cert-v01@openssh.com".into(),
            serial: 67890,
            valid_from: "2024-06-01 00:00:00".into(),
            valid_to: "2025-06-01 00:00:00".into(),
            is_valid: false,
            ca_fingerprint: "SHA256:CA9qR8sT7uV6wX5yZ4aB3cD2eF".into(),
            key_id: "deploy@ci-2024".into(),
            principals: vec!["deploy".into()],
        },
        CertificateEntry {
            name: "bastion-host-cert.pub".into(),
            cert_type: "Host".into(),
            key_type: "ssh-rsa-cert-v01@openssh.com".into(),
            serial: 42,
            valid_from: "2025-03-01 00:00:00".into(),
            valid_to: "2026-03-01 00:00:00".into(),
            is_valid: true,
            ca_fingerprint: "SHA256:CA2gH3iJ4kL5mN6oP7qR8sT9uV".into(),
            key_id: "bastion.example.com".into(),
            principals: vec!["bastion.example.com".into()],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_not_pending() {
        let collector = SshDataCollector::new();
        assert!(!collector.is_pending());
    }

    #[test]
    fn default_matches_new() {
        assert_eq!(
            SshDataCollector::new().is_pending(),
            SshDataCollector::default().is_pending()
        );
    }

    #[tokio::test]
    async fn start_makes_pending() {
        let mut collector = SshDataCollector::new();
        assert!(!collector.is_pending());
        collector.start();
        assert!(collector.is_pending());
    }

    #[tokio::test]
    async fn start_is_idempotent() {
        let mut collector = SshDataCollector::new();
        collector.start();
        collector.start();
        assert!(collector.is_pending());
    }

    #[tokio::test]
    async fn poll_returns_bundle_after_collection() {
        let mut collector = SshDataCollector::new();
        collector.start();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let result = collector.poll().await;
        assert!(result.is_some());
        let bundle = result.unwrap();
        assert_eq!(bundle.keys.len(), 3);
        assert_eq!(bundle.known_hosts.len(), 5);
        assert_eq!(bundle.config_hosts.len(), 5);
        assert_eq!(bundle.agent_keys.len(), 3);
        assert_eq!(bundle.forwarding.len(), 2);
        assert_eq!(bundle.diagnostics.len(), 6);
        assert_eq!(bundle.authorized_keys.len(), 4);
        assert_eq!(bundle.certificates.len(), 3);
    }

    #[tokio::test]
    async fn poll_clears_pending() {
        let mut collector = SshDataCollector::new();
        collector.start();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let _ = collector.poll().await;
        assert!(!collector.is_pending());
    }

    #[tokio::test]
    async fn poll_returns_none_when_not_started() {
        let mut collector = SshDataCollector::new();
        let result = collector.poll().await;
        assert!(result.is_none());
    }

    #[test]
    fn mock_data_have_expected_content() {
        let bundle = collect_mock_data();
        assert!(!bundle.keys.is_empty());
        assert!(!bundle.known_hosts.is_empty());
        assert!(!bundle.config_hosts.is_empty());
        assert!(!bundle.agent_keys.is_empty());
        assert!(!bundle.forwarding.is_empty());
        assert!(!bundle.diagnostics.is_empty());
        assert!(!bundle.authorized_keys.is_empty());
        assert!(!bundle.certificates.is_empty());
        assert!(bundle.agent_status.reachable);
    }
}
