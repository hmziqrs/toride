use super::*;
use crate::config::ast::{parse as parse_ast, ConfigAst};

fn make_ast(input: &str) -> ConfigAst {
    parse_ast(input)
}

#[test]
fn get_directive_finds_hostname() {
    let ast = make_ast(
        "\
Host example
    HostName example.com
    User alice
",
    );
    let val = get_directive(&ast, "example", "HostName");
    assert_eq!(val, Some("example.com".to_owned()));
}

#[test]
fn get_directive_case_insensitive_key() {
    let ast = make_ast(
        "\
Host example
    hostname example.com
",
    );
    let val = get_directive(&ast, "example", "HOSTNAME");
    assert_eq!(val, Some("example.com".to_owned()));
}

#[test]
fn get_directive_wildcard_match() {
    let ast = make_ast(
        "\
Host *
    User default
",
    );
    let val = get_directive(&ast, "anything", "User");
    assert_eq!(val, Some("default".to_owned()));
}

#[test]
fn get_directive_negation() {
    let ast = make_ast(
        "\
Host * !badhost
    User default
",
    );
    let val = get_directive(&ast, "goodhost", "User");
    assert_eq!(val, Some("default".to_owned()));

    let val = get_directive(&ast, "badhost", "User");
    assert_eq!(val, None);
}

#[test]
fn glob_matches_works() {
    assert!(glob_matches("example.com", "example.com"));
    assert!(glob_matches("anything", "*"));
    assert!(glob_matches("foo.example.com", "*.example.com"));
    assert!(glob_matches("a", "?"));
    assert!(!glob_matches("example.com", "other.com"));
    assert!(!glob_matches("example.com", "*.org"));
}

#[test]
fn glob_wildcard_excludes_bare_domain() {
    assert!(!glob_matches("example.com", "*.example.com"));
    assert!(glob_matches("sub.example.com", "*.example.com"));
}

#[test]
fn glob_matches_case_insensitive() {
    assert!(host_matches_patterns("Example.COM", &["example.com".to_owned()]));
}

#[test]
fn accumulative_directives_collected_across_blocks() {
    let ast = make_ast(
        "\
Host myhost
    IdentityFile ~/.ssh/id_ed25519

Host *
    IdentityFile ~/.ssh/id_rsa
",
    );
    let vals = get_accumulative_directive(&ast, "myhost", "IdentityFile");
    assert_eq!(vals.len(), 2);
    assert_eq!(vals[0], "~/.ssh/id_ed25519");
    assert_eq!(vals[1], "~/.ssh/id_rsa");
}

#[test]
fn is_accumulative_known_directives() {
    assert!(is_accumulative("identityfile"));
    assert!(is_accumulative("certificatefile"));
    assert!(is_accumulative("proxyjump"));
    assert!(is_accumulative("forwardagent"));
    assert!(!is_accumulative("hostname"));
    assert!(!is_accumulative("user"));
    assert!(!is_accumulative("port"));
}

// ---------------------------------------------------------------------------
// Edge-case tests for glob and pattern matching
// ---------------------------------------------------------------------------

#[test]
fn glob_matches_empty_text_and_pattern() {
    assert!(glob_matches("", ""));
    assert!(glob_matches("", "*"));
    assert!(!glob_matches("", "?"));
    assert!(!glob_matches("", "a"));
}

#[test]
fn glob_matches_question_mark_edge_cases() {
    assert!(glob_matches("a", "?"));
    assert!(!glob_matches("", "?"));
    assert!(glob_matches("ab", "??"));
    assert!(!glob_matches("a", "??"));
}

#[test]
fn glob_matches_multiple_stars() {
    assert!(glob_matches("anything", "**"));
    assert!(glob_matches("anything", "***"));
    assert!(glob_matches("a.b.c", "*.*.*"));
    assert!(!glob_matches("ab", "*.*.*"));
}

#[test]
fn glob_matches_star_at_start() {
    assert!(glob_matches("example.com", "*.com"));
    assert!(!glob_matches("com", "*.com"));
}

#[test]
fn glob_matches_star_in_middle() {
    assert!(glob_matches("sub.example.com", "sub.*.com"));
    assert!(!glob_matches("sub.example.org", "sub.*.com"));
}

#[test]
fn host_matches_patterns_empty_patterns() {
    assert!(!host_matches_patterns("example.com", &[]));
}

#[test]
fn host_matches_patterns_only_negation() {
    // Only negation patterns with no positive match should return false
    let patterns = vec!["!badhost".to_string()];
    assert!(!host_matches_patterns("goodhost", &patterns));
    assert!(!host_matches_patterns("badhost", &patterns));
}

#[test]
fn host_matches_patterns_negation_overrides_positive() {
    let patterns = vec!["*".to_string(), "!badhost".to_string()];
    assert!(host_matches_patterns("goodhost", &patterns));
    assert!(!host_matches_patterns("badhost", &patterns));
}

#[test]
fn host_matches_patterns_case_insensitive() {
    assert!(host_matches_patterns("EXAMPLE.COM", &["example.com".to_string()]));
    assert!(host_matches_patterns("example.com", &["EXAMPLE.COM".to_string()]));
}

#[test]
fn host_matches_patterns_wildcard_all() {
    let patterns = vec!["*".to_string()];
    assert!(host_matches_patterns("anything", &patterns));
    assert!(host_matches_patterns("", &patterns));
}

#[test]
fn host_matches_patterns_empty_host() {
    assert!(host_matches_patterns("", &["*".to_string()]));
    assert!(!host_matches_patterns("", &["example.com".to_string()]));
}

#[test]
fn get_directive_no_matching_host() {
    let ast = make_ast("Host other\n    User alice\n");
    let val = get_directive(&ast, "example", "User");
    assert_eq!(val, None);
}

#[test]
fn get_directive_empty_ast() {
    let ast = make_ast("");
    let val = get_directive(&ast, "example", "User");
    assert_eq!(val, None);
}

#[test]
fn get_directive_first_match_wins() {
    let ast = make_ast(
        "\
Host *
    User first

Host *
    User second
",
    );
    let val = get_directive(&ast, "anything", "User");
    assert_eq!(val, Some("first".to_owned()));
}

#[test]
fn get_accumulative_directive_no_match() {
    let ast = make_ast("Host other\n    IdentityFile ~/.ssh/id_rsa\n");
    let vals = get_accumulative_directive(&ast, "example", "IdentityFile");
    assert!(vals.is_empty());
}

#[test]
fn set_directive_creates_new_block() {
    let mut ast = make_ast("Host example\n    User alice\n");
    set_directive(&mut ast, "example", "HostName", "example.com").unwrap();
    let val = get_directive(&ast, "example", "HostName");
    assert_eq!(val, Some("example.com".to_owned()));
}

#[test]
fn set_directive_updates_existing() {
    let mut ast = make_ast("Host example\n    HostName old.com\n");
    set_directive(&mut ast, "example", "HostName", "new.com").unwrap();
    let val = get_directive(&ast, "example", "HostName");
    assert_eq!(val, Some("new.com".to_owned()));
}

#[test]
fn set_directive_host_not_found() {
    let mut ast = make_ast("");
    assert!(set_directive(&mut ast, "missing", "User", "alice").is_err());
}
