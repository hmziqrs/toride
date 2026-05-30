use super::*;
use clap::Parser;
use std::path::PathBuf;

// -----------------------------------------------------------------------
// Start command
// -----------------------------------------------------------------------

#[test]
fn parse_start_command() {
    let cli = Cli::try_parse_from(["toride-fail2ban", "start"]).unwrap();
    assert!(matches!(cli.command, Commands::Start { jail: None }));
}

#[test]
fn parse_start_with_jail() {
    let cli = Cli::try_parse_from(["toride-fail2ban", "start", "--jail", "sshd"]).unwrap();
    match cli.command {
        Commands::Start { jail } => assert_eq!(jail.as_deref(), Some("sshd")),
        _ => panic!("expected Start command"),
    }
}

// -----------------------------------------------------------------------
// Stop command
// -----------------------------------------------------------------------

#[test]
fn parse_stop_command() {
    let cli = Cli::try_parse_from(["toride-fail2ban", "stop"]).unwrap();
    assert!(matches!(cli.command, Commands::Stop));
}

// -----------------------------------------------------------------------
// Status command
// -----------------------------------------------------------------------

#[test]
fn parse_status_command() {
    let cli = Cli::try_parse_from(["toride-fail2ban", "status"]).unwrap();
    assert!(matches!(cli.command, Commands::Status { jail: None }));
}

#[test]
fn parse_status_with_jail_arg() {
    let cli = Cli::try_parse_from(["toride-fail2ban", "status", "sshd"]).unwrap();
    match cli.command {
        Commands::Status { jail } => assert_eq!(jail.as_deref(), Some("sshd")),
        _ => panic!("expected Status command"),
    }
}

// -----------------------------------------------------------------------
// Ban command
// -----------------------------------------------------------------------

#[test]
fn parse_ban_with_ip() {
    let cli = Cli::try_parse_from(["toride-fail2ban", "ban", "192.168.1.100"]).unwrap();
    match cli.command {
        Commands::Ban { ip, jail } => {
            assert_eq!(ip, "192.168.1.100".parse::<std::net::IpAddr>().unwrap());
            assert_eq!(jail, "default");
        }
        _ => panic!("expected Ban command"),
    }
}

#[test]
fn parse_ban_with_jail_flag() {
    let cli = Cli::try_parse_from([
        "toride-fail2ban",
        "ban",
        "10.0.0.1",
        "--jail",
        "sshd",
    ])
    .unwrap();
    match cli.command {
        Commands::Ban { ip, jail } => {
            assert_eq!(ip, "10.0.0.1".parse::<std::net::IpAddr>().unwrap());
            assert_eq!(jail, "sshd");
        }
        _ => panic!("expected Ban command"),
    }
}

// -----------------------------------------------------------------------
// Unban command
// -----------------------------------------------------------------------

#[test]
fn parse_unban_with_ip() {
    let cli = Cli::try_parse_from(["toride-fail2ban", "unban", "192.168.1.100"]).unwrap();
    match cli.command {
        Commands::Unban { ip, jail } => {
            assert_eq!(ip, "192.168.1.100".parse::<std::net::IpAddr>().unwrap());
            assert_eq!(jail, "default");
        }
        _ => panic!("expected Unban command"),
    }
}

#[test]
fn parse_unban_with_jail_flag() {
    let cli = Cli::try_parse_from([
        "toride-fail2ban",
        "unban",
        "10.0.0.1",
        "--jail",
        "nginx",
    ])
    .unwrap();
    match cli.command {
        Commands::Unban { ip, jail } => {
            assert_eq!(ip, "10.0.0.1".parse::<std::net::IpAddr>().unwrap());
            assert_eq!(jail, "nginx");
        }
        _ => panic!("expected Unban command"),
    }
}

// -----------------------------------------------------------------------
// Set command
// -----------------------------------------------------------------------

#[test]
fn parse_set_command() {
    let cli = Cli::try_parse_from([
        "toride-fail2ban",
        "set",
        "sshd",
        "maxretry",
        "10",
    ])
    .unwrap();
    match cli.command {
        Commands::Set { jail, param, value } => {
            assert_eq!(jail, "sshd");
            assert_eq!(param, "maxretry");
            assert_eq!(value, "10");
        }
        _ => panic!("expected Set command"),
    }
}

// -----------------------------------------------------------------------
// Test command
// -----------------------------------------------------------------------

#[test]
fn parse_test_with_pattern() {
    let cli = Cli::try_parse_from([
        "toride-fail2ban",
        "test",
        "/var/log/auth.log",
        "--pattern",
        r#"Failed password for .* from (\S+)"#,
    ])
    .unwrap();
    match cli.command {
        Commands::Test { log_path, pattern } => {
            assert_eq!(log_path, PathBuf::from("/var/log/auth.log"));
            assert_eq!(pattern, r#"Failed password for .* from (\S+)"#);
        }
        _ => panic!("expected Test command"),
    }
}

// -----------------------------------------------------------------------
// AddJail command
// -----------------------------------------------------------------------

#[test]
fn parse_addjail_with_all_options() {
    let cli = Cli::try_parse_from([
        "toride-fail2ban",
        "add-jail",
        "sshd",
        "--log-path",
        "/var/log/auth.log",
        "--pattern",
        r#"Failed password"#,
        "--max-retry",
        "3",
        "--ban-time",
        "7200",
    ])
    .unwrap();
    match cli.command {
        Commands::AddJail {
            name,
            log_path,
            pattern,
            max_retry,
            ban_time,
        } => {
            assert_eq!(name, "sshd");
            assert_eq!(log_path, PathBuf::from("/var/log/auth.log"));
            assert_eq!(pattern, r#"Failed password"#);
            assert_eq!(max_retry, 3);
            assert_eq!(ban_time, 7200);
        }
        _ => panic!("expected AddJail command"),
    }
}

#[test]
fn parse_addjail_defaults() {
    let cli = Cli::try_parse_from([
        "toride-fail2ban",
        "add-jail",
        "nginx",
        "--log-path",
        "/var/log/nginx/error.log",
        "--pattern",
        r#"limiting requests"#,
    ])
    .unwrap();
    match cli.command {
        Commands::AddJail {
            name,
            max_retry,
            ban_time,
            ..
        } => {
            assert_eq!(name, "nginx");
            assert_eq!(max_retry, 5);
            assert_eq!(ban_time, 3600);
        }
        _ => panic!("expected AddJail command"),
    }
}

// -----------------------------------------------------------------------
// RmJail command
// -----------------------------------------------------------------------

#[test]
fn parse_rmjail() {
    let cli = Cli::try_parse_from(["toride-fail2ban", "rm-jail", "sshd"]).unwrap();
    match cli.command {
        Commands::RmJail { name } => assert_eq!(name, "sshd"),
        _ => panic!("expected RmJail command"),
    }
}

// -----------------------------------------------------------------------
// Global flags
// -----------------------------------------------------------------------

#[test]
fn parse_with_config_flag() {
    let cli = Cli::try_parse_from([
        "toride-fail2ban",
        "--config",
        "/etc/toride/f2b.json",
        "status",
    ])
    .unwrap();
    assert_eq!(cli.config, PathBuf::from("/etc/toride/f2b.json"));
}

#[test]
fn parse_with_verbose_flag() {
    let cli = Cli::try_parse_from(["toride-fail2ban", "--verbose", "status"]).unwrap();
    assert!(cli.verbose);
}

#[test]
fn parse_with_dry_run_flag() {
    let cli = Cli::try_parse_from(["toride-fail2ban", "--dry-run", "start"]).unwrap();
    assert!(cli.dry_run);
}

// -----------------------------------------------------------------------
// Defaults
// -----------------------------------------------------------------------

#[test]
fn default_config_path() {
    let cli = Cli::try_parse_from(["toride-fail2ban", "status"]).unwrap();
    assert_eq!(
        cli.config,
        PathBuf::from("~/.config/toride/fail2ban/config.json")
    );
}

#[test]
fn default_jail_for_ban_is_default() {
    let cli = Cli::try_parse_from(["toride-fail2ban", "ban", "127.0.0.1"]).unwrap();
    match cli.command {
        Commands::Ban { jail, .. } => assert_eq!(jail, "default"),
        _ => panic!("expected Ban command"),
    }
}

#[test]
fn default_jail_for_unban_is_default() {
    let cli = Cli::try_parse_from(["toride-fail2ban", "unban", "127.0.0.1"]).unwrap();
    match cli.command {
        Commands::Unban { jail, .. } => assert_eq!(jail, "default"),
        _ => panic!("expected Unban command"),
    }
}
