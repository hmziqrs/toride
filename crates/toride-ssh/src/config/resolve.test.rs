use super::*;

#[test]
fn expand_env_vars_no_vars() {
    assert_eq!(expand_env_vars("hello"), "hello");
}

#[test]
fn expand_env_vars_simple_var() {
    // SAFETY: tests run with `--test-threads=1` by default in this crate,
    // and we use unique var names to avoid cross-test interference.
    unsafe {
        std::env::set_var("TORIDE_TEST_VAR", "world");
    }
    assert_eq!(expand_env_vars("${TORIDE_TEST_VAR}"), "world");
    unsafe {
        std::env::remove_var("TORIDE_TEST_VAR");
    }
}

#[test]
fn expand_env_vars_undefined_var() {
    assert_eq!(expand_env_vars("${TORIDE_UNDEFINED_VAR_XYZ}"), "");
}

#[test]
fn expand_env_vars_unclosed_brace() {
    // Unclosed ${ — the $ is consumed but the rest is preserved
    assert_eq!(expand_env_vars("${UNCLOSED"), "$UNCLOSED");
}

#[test]
fn expand_env_vars_mixed_text() {
    unsafe {
        std::env::set_var("TORIDE_TEST_HOST", "example.com");
    }
    assert_eq!(
        expand_env_vars("Host: ${TORIDE_TEST_HOST}, Port: 22"),
        "Host: example.com, Port: 22"
    );
    unsafe {
        std::env::remove_var("TORIDE_TEST_HOST");
    }
}

#[test]
fn expand_env_vars_multiple_vars() {
    unsafe {
        std::env::set_var("TORIDE_A", "alpha");
        std::env::set_var("TORIDE_B", "beta");
    }
    assert_eq!(
        expand_env_vars("${TORIDE_A}-${TORIDE_B}"),
        "alpha-beta"
    );
    unsafe {
        std::env::remove_var("TORIDE_A");
        std::env::remove_var("TORIDE_B");
    }
}

#[test]
fn expand_env_vars_empty_string() {
    assert_eq!(expand_env_vars(""), "");
}

#[test]
fn expand_env_vars_only_dollar() {
    assert_eq!(expand_env_vars("$"), "$");
}

#[test]
fn expand_tilde_home_dir() {
    let result = expand_tilde_and_env("~/test");
    // Should start with the home directory path
    assert!(!result.starts_with("~/"));
    assert!(result.ends_with("/test"));
}

#[test]
fn expand_tilde_no_tilde() {
    assert_eq!(expand_tilde_and_env("/absolute/path"), "/absolute/path");
}

#[test]
fn expand_tilde_just_tilde() {
    let result = expand_tilde_and_env("~");
    // Should expand to home dir
    assert!(!result.is_empty());
    assert!(!result.starts_with("~"));
}

#[test]
fn collapse_double_percent_basic() {
    assert_eq!(collapse_double_percent("%%"), "%");
    assert_eq!(collapse_double_percent("hello%%world"), "hello%world");
    assert_eq!(collapse_double_percent("no percent"), "no percent");
}

#[test]
fn collapse_double_percent_multiple() {
    assert_eq!(collapse_double_percent("%%%%"), "%%");
}

#[test]
fn expand_tokens_no_tokens() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22" };
    assert_eq!(expand_tokens("hello", &ctx), "hello");
}

#[test]
fn expand_tokens_host() {
    let ctx = TokenContext { host: "example.com", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22" };
    assert_eq!(expand_tokens("%h", &ctx), "example.com");
}

#[test]
fn expand_tokens_user() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "alice", port: "22" };
    assert_eq!(expand_tokens("%u", &ctx), "alice");
}

#[test]
fn expand_tokens_port() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "2222" };
    assert_eq!(expand_tokens("%p", &ctx), "2222");
}

#[test]
fn expand_tokens_home_dir() {
    let ctx = TokenContext { host: "host", home_dir: "/home/alice", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22" };
    assert_eq!(expand_tokens("%d", &ctx), "/home/alice");
}

#[test]
fn expand_tokens_local_hostname() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "myhost", remote_user: "remote", local_user: "user", port: "22" };
    assert_eq!(expand_tokens("%l", &ctx), "myhost");
}

#[test]
fn expand_tokens_remote_user() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "deploy", local_user: "user", port: "22" };
    assert_eq!(expand_tokens("%r", &ctx), "deploy");
}

#[test]
fn expand_tokens_unknown_token() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22" };
    assert_eq!(expand_tokens("%z", &ctx), "%z");
}

#[test]
fn expand_tokens_trailing_percent() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22" };
    assert_eq!(expand_tokens("hello%", &ctx), "hello%");
}

#[test]
fn expand_tokens_double_percent() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22" };
    assert_eq!(expand_tokens("%%", &ctx), "%%");
}

#[test]
fn expand_tokens_mixed() {
    let ctx = TokenContext { host: "example.com", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22" };
    assert_eq!(expand_tokens("%h:%p", &ctx), "example.com:22");
}

#[test]
fn simple_glob_match_exact() {
    assert!(simple_glob_match("test.conf", "test.conf"));
    assert!(!simple_glob_match("test.conf", "other.conf"));
}

#[test]
fn simple_glob_match_star() {
    assert!(simple_glob_match("anything", "*"));
}

#[test]
fn simple_glob_match_no_wildcards() {
    assert!(simple_glob_match("abc", "abc"));
    assert!(!simple_glob_match("abc", "def"));
}

#[test]
fn match_criteria_host_basic() {
    assert!(match_criteria_host("host web", "web"));
    assert!(!match_criteria_host("host web", "db"));
}

#[test]
fn match_criteria_host_no_host_clause() {
    assert!(!match_criteria_host("user alice", "web"));
}

#[test]
fn match_criteria_host_wildcard() {
    assert!(match_criteria_host("host *", "anything"));
}

#[test]
fn match_criteria_host_negation() {
    // Negation patterns must be comma-separated with positive patterns
    assert!(!match_criteria_host("host *,!badhost", "badhost"));
    assert!(match_criteria_host("host *,!badhost", "goodhost"));
}
