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
    // Unclosed ${ — the literal ${ is preserved to avoid path corruption.
    assert_eq!(expand_env_vars("${UNCLOSED"), "${UNCLOSED");
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
fn expand_env_vars_no_braces() {
    unsafe {
        std::env::set_var("TORIDE_NO_BRACE", "nobrace");
    }
    assert_eq!(expand_env_vars("$TORIDE_NO_BRACE"), "nobrace");
    unsafe {
        std::env::remove_var("TORIDE_NO_BRACE");
    }
}

#[test]
fn expand_env_vars_no_braces_with_suffix() {
    unsafe {
        std::env::set_var("TORIDE_VAR_SUF", "value");
    }
    assert_eq!(expand_env_vars("${TORIDE_VAR_SUF}_extra"), "value_extra");
    assert_eq!(expand_env_vars("$TORIDE_VAR_SUF.extra"), "value.extra");
    unsafe {
        std::env::remove_var("TORIDE_VAR_SUF");
    }
}

#[test]
fn expand_env_vars_no_braces_undefined() {
    // Undefined $VAR should expand to empty string.
    assert_eq!(expand_env_vars("$TORIDE_UNDEF_XYZ_123"), "");
}

#[test]
fn expand_env_vars_bare_dollar_before_non_name() {
    // `$ ` (dollar followed by space) should preserve the dollar.
    assert_eq!(expand_env_vars("$ "), "$ ");
    assert_eq!(expand_env_vars("$/path"), "$/path");
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
    assert!(!result.starts_with('~'));
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
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22", canonical_host: "host" };
    assert_eq!(expand_tokens("hello", &ctx), "hello");
}

#[test]
fn expand_tokens_host() {
    let ctx = TokenContext { host: "example.com", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22", canonical_host: "example.com" };
    assert_eq!(expand_tokens("%h", &ctx), "example.com");
}

#[test]
fn expand_tokens_user() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "alice", port: "22", canonical_host: "host" };
    assert_eq!(expand_tokens("%u", &ctx), "alice");
}

#[test]
fn expand_tokens_port() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "2222", canonical_host: "host" };
    assert_eq!(expand_tokens("%p", &ctx), "2222");
}

#[test]
fn expand_tokens_home_dir() {
    let ctx = TokenContext { host: "host", home_dir: "/home/alice", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22", canonical_host: "host" };
    assert_eq!(expand_tokens("%d", &ctx), "/home/alice");
}

#[test]
fn expand_tokens_local_hostname() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "myhost", remote_user: "remote", local_user: "user", port: "22", canonical_host: "host" };
    assert_eq!(expand_tokens("%l", &ctx), "myhost");
}

#[test]
fn expand_tokens_remote_user() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "deploy", local_user: "user", port: "22", canonical_host: "host" };
    assert_eq!(expand_tokens("%r", &ctx), "deploy");
}

#[test]
fn expand_tokens_unknown_token() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22", canonical_host: "host" };
    assert_eq!(expand_tokens("%z", &ctx), "%z");
}

#[test]
fn expand_tokens_trailing_percent() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22", canonical_host: "host" };
    assert_eq!(expand_tokens("hello%", &ctx), "hello%");
}

#[test]
fn expand_tokens_double_percent() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22", canonical_host: "host" };
    assert_eq!(expand_tokens("%%", &ctx), "%%");
}

#[test]
fn expand_tokens_mixed() {
    let ctx = TokenContext { host: "example.com", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22", canonical_host: "example.com" };
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
    assert!(match_criteria_host("host web", "web", "alice", "web"));
    assert!(!match_criteria_host("host web", "db", "alice", "web"));
}

#[test]
fn match_criteria_host_no_host_clause() {
    // "user alice" with target_user "bob" should not match.
    assert!(!match_criteria_host("user alice", "web", "bob", "web"));
    // "user alice" with target_user "alice" should match.
    assert!(match_criteria_host("user alice", "web", "alice", "web"));
}

#[test]
fn match_criteria_host_wildcard() {
    assert!(match_criteria_host("host *", "anything", "alice", "anything"));
}

// ---------------------------------------------------------------------------
// Weird edge-case tests for token expansion
// ---------------------------------------------------------------------------

#[test]
fn expand_tokens_consecutive_tokens() {
    let ctx = TokenContext { host: "h", home_dir: "d", local_hostname: "l", remote_user: "r", local_user: "u", port: "p", canonical_host: "h" };
    assert_eq!(expand_tokens("%h%h%h", &ctx), "hhh");
}

#[test]
fn expand_tokens_adjacent_different_tokens() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22", canonical_host: "host" };
    assert_eq!(expand_tokens("%h:%p:%u", &ctx), "host:22:user");
}

#[test]
fn expand_tokens_percent_at_end() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22", canonical_host: "host" };
    assert_eq!(expand_tokens("hello%", &ctx), "hello%");
}

#[test]
fn expand_tokens_percent_at_start() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22", canonical_host: "host" };
    // %h is a valid token (host), so %hello becomes "hostello"
    assert_eq!(expand_tokens("%h", &ctx), "host");
}

#[test]
fn expand_tokens_only_percent() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22", canonical_host: "host" };
    assert_eq!(expand_tokens("%", &ctx), "%");
}

#[test]
fn expand_tokens_escaped_percent_sequence() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22", canonical_host: "host" };
    // %%h should produce %%h (collapse_double_percent handles %% → %)
    assert_eq!(expand_tokens("%%h", &ctx), "%%h");
}

#[test]
fn expand_tokens_empty_string() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22", canonical_host: "host" };
    assert_eq!(expand_tokens("", &ctx), "");
}

#[test]
fn expand_tokens_no_tokens_complex() {
    let ctx = TokenContext { host: "host", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22", canonical_host: "host" };
    assert_eq!(expand_tokens("/usr/local/bin/ssh", &ctx), "/usr/local/bin/ssh");
}

#[test]
fn expand_tilde_just_slash() {
    // ~/ should expand to home dir + /
    let result = expand_tilde_and_env("~/");
    assert!(!result.starts_with("~/"));
    assert!(result.ends_with('/'));
}

#[test]
fn expand_tilde_path_with_spaces() {
    // This is unusual but should work
    let result = expand_tilde_and_env("~/path with spaces");
    assert!(!result.starts_with("~/"));
    assert!(result.ends_with("/path with spaces"));
}

#[test]
fn expand_env_vars_var_at_start() {
    unsafe { std::env::set_var("TORIDE_TEST_START", "hello"); }
    assert_eq!(expand_env_vars("${TORIDE_TEST_START}world"), "helloworld");
    unsafe { std::env::remove_var("TORIDE_TEST_START"); }
}

#[test]
fn expand_env_vars_var_at_end() {
    unsafe { std::env::set_var("TORIDE_TEST_END", "world"); }
    assert_eq!(expand_env_vars("hello${TORIDE_TEST_END}"), "helloworld");
    unsafe { std::env::remove_var("TORIDE_TEST_END"); }
}

#[test]
fn expand_env_vars_adjacent_vars() {
    unsafe {
        std::env::set_var("TORIDE_X", "X");
        std::env::set_var("TORIDE_Y", "Y");
    }
    assert_eq!(expand_env_vars("${TORIDE_X}${TORIDE_Y}"), "XY");
    unsafe {
        std::env::remove_var("TORIDE_X");
        std::env::remove_var("TORIDE_Y");
    }
}

#[test]
fn expand_env_vars_nested_braces() {
    // ${${VAR}} — the inner ${ is consumed, leaving $VAR}
    // This is unusual but should not panic
    let result = expand_env_vars("${${NESTED}}");
    let _ = result; // just check it doesn't panic
}

#[test]
fn match_criteria_host_case_insensitive_keyword() {
    assert!(match_criteria_host("HOST web", "web", "alice", "web"));
    assert!(match_criteria_host("Host web", "web", "alice", "web"));
    assert!(match_criteria_host("host web", "web", "alice", "web"));
}

#[test]
fn match_criteria_host_multiple_host_clauses() {
    // Multiple host clauses - either can match
    assert!(match_criteria_host("host web host db", "web", "alice", "web"));
    assert!(match_criteria_host("host web host db", "db", "alice", "web"));
    assert!(!match_criteria_host("host web host db", "other", "alice", "web"));
}

#[test]
fn match_criteria_host_unknown_keyword_before_host() {
    // Unknown keyword before host clause — user "alice" must also match.
    assert!(match_criteria_host("user alice host web", "web", "alice", "web"));
    // If user doesn't match, the whole block is rejected.
    assert!(!match_criteria_host("user alice host web", "web", "bob", "web"));
}

#[test]
fn collapse_double_percent_empty() {
    assert_eq!(collapse_double_percent(""), "");
}

#[test]
fn collapse_double_percent_single_percent() {
    assert_eq!(collapse_double_percent("%"), "%");
}

#[test]
fn collapse_double_percent_triple() {
    // %%% should become %% (one pair collapsed, one left)
    assert_eq!(collapse_double_percent("%%%"), "%%");
}

// ---------------------------------------------------------------------------
// Production-grade weird edge cases
// ---------------------------------------------------------------------------

#[test]
fn expand_env_vars_with_spaces_in_value() {
    unsafe { std::env::set_var("TORIDE_SPACED", "hello world"); }
    assert_eq!(expand_env_vars("${TORIDE_SPACED}"), "hello world");
    unsafe { std::env::remove_var("TORIDE_SPACED"); }
}

#[test]
fn expand_env_vars_with_equals_in_value() {
    unsafe { std::env::set_var("TORIDE_EQUALS", "a=b"); }
    assert_eq!(expand_env_vars("${TORIDE_EQUALS}"), "a=b");
    unsafe { std::env::remove_var("TORIDE_EQUALS"); }
}

#[test]
fn expand_env_vars_with_special_chars() {
    unsafe { std::env::set_var("TORIDE_SPECIAL", "hello@world.com"); }
    assert_eq!(expand_env_vars("${TORIDE_SPECIAL}"), "hello@world.com");
    unsafe { std::env::remove_var("TORIDE_SPECIAL"); }
}

#[test]
fn expand_env_vars_undefined_returns_empty() {
    // Undefined vars should expand to empty string
    assert_eq!(expand_env_vars("${TORIDE_DEFINITELY_NOT_SET_12345}"), "");
}

#[test]
fn expand_env_vars_with_dollar_sign_in_value() {
    // Dollar sign in env var value should not be re-expanded
    unsafe { std::env::set_var("TORIDE_DOLLAR", "$NOT_A_VAR"); }
    assert_eq!(expand_env_vars("${TORIDE_DOLLAR}"), "$NOT_A_VAR");
    unsafe { std::env::remove_var("TORIDE_DOLLAR"); }
}

#[test]
fn expand_tilde_with_trailing_slash() {
    let result = expand_tilde_and_env("~/");
    assert!(!result.starts_with("~/"));
    assert!(result.ends_with('/'));
}

#[test]
fn expand_tilde_with_deep_path() {
    let result = expand_tilde_and_env("~/.ssh/keys/backup");
    assert!(!result.starts_with("~/"));
    assert!(result.ends_with("/.ssh/keys/backup"));
}

#[test]
fn expand_tilde_not_at_start() {
    // Tilde not at start should not be expanded
    assert_eq!(expand_tilde_and_env("path/~user"), "path/~user");
}

#[test]
fn expand_tokens_all_tokens_combined() {
    let ctx = TokenContext {
        host: "example.com",
        home_dir: "/home/alice",
        local_hostname: "myhost",
        remote_user: "deploy",
        local_user: "alice",
        port: "2222",
        canonical_host: "example.com",
    };
    let result = expand_tokens("%h:%p:%u:%d:%l:%r", &ctx);
    assert_eq!(result, "example.com:2222:alice:/home/alice:myhost:deploy");
}

#[test]
fn expand_tokens_with_path() {
    let ctx = TokenContext { host: "h", home_dir: "/home", local_hostname: "l", remote_user: "r", local_user: "u", port: "22", canonical_host: "h" };
    assert_eq!(expand_tokens("/keys/%h/%u", &ctx), "/keys/h/u");
}

#[test]
fn expand_tokens_with_env_var() {
    let ctx = TokenContext { host: "h", home_dir: "/home", local_hostname: "l", remote_user: "r", local_user: "u", port: "22", canonical_host: "h" };
    // %d is home_dir, not an env var
    assert_eq!(expand_tokens("%d/.ssh", &ctx), "/home/.ssh");
}

#[test]
fn expand_tokens_canonical_host() {
    let ctx = TokenContext { host: "alias", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "user", port: "22", canonical_host: "real.host.com" };
    assert_eq!(expand_tokens("%H", &ctx), "real.host.com");
}

#[test]
fn expand_tokens_local_user_short() {
    let ctx = TokenContext { host: "h", home_dir: "/home", local_hostname: "local", remote_user: "remote", local_user: "alice", port: "22", canonical_host: "h" };
    assert_eq!(expand_tokens("%i", &ctx), "alice");
}

#[test]
fn expand_tokens_local_hostname_short() {
    let ctx = TokenContext { host: "h", home_dir: "/home", local_hostname: "myhost.example.com", remote_user: "remote", local_user: "user", port: "22", canonical_host: "h" };
    // %L should give short hostname (before first '.')
    assert_eq!(expand_tokens("%L", &ctx), "myhost");
}

#[test]
fn expand_tokens_local_hostname_short_no_dot() {
    let ctx = TokenContext { host: "h", home_dir: "/home", local_hostname: "myhost", remote_user: "remote", local_user: "user", port: "22", canonical_host: "h" };
    assert_eq!(expand_tokens("%L", &ctx), "myhost");
}

#[test]
fn expand_tokens_remote_user_alias() {
    // %T is an alias for %r (remote user)
    let ctx = TokenContext { host: "h", home_dir: "/home", local_hostname: "l", remote_user: "deploy", local_user: "u", port: "22", canonical_host: "h" };
    assert_eq!(expand_tokens("%T", &ctx), "deploy");
}

#[test]
fn expand_tokens_connection_hash() {
    let ctx = TokenContext { host: "h", home_dir: "/home", local_hostname: "l", remote_user: "r", local_user: "u", port: "22", canonical_host: "h" };
    let hash = expand_tokens("%C", &ctx);
    // %C should produce a hex string
    assert!(!hash.is_empty());
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn expand_tokens_all_new_tokens() {
    let ctx = TokenContext { host: "alias", home_dir: "/home/alice", local_hostname: "myhost.example.com", remote_user: "deploy", local_user: "alice", port: "2222", canonical_host: "real.com" };
    // Test all new tokens in one go
    let result = expand_tokens("%H:%i:%L:%T:%C", &ctx);
    let parts: Vec<&str> = result.split(':').collect();
    assert_eq!(parts[0], "real.com");       // %H canonical
    assert_eq!(parts[1], "alice");           // %i local user
    assert_eq!(parts[2], "myhost");          // %L short hostname
    assert_eq!(parts[3], "deploy");          // %T remote user
    assert!(!parts[4].is_empty());           // %C hash
}

#[test]
fn match_criteria_host_with_port() {
    // Match criteria with port in host pattern
    assert!(match_criteria_host("host [::1]:22", "[::1]:22", "alice", "[::1]:22"));
}

#[test]
fn match_criteria_host_with_wildcard_port() {
    // Wildcard host should match any port
    assert!(match_criteria_host("host *", "example.com:22", "alice", "example.com:22"));
}

#[test]
fn match_criteria_host_empty_criteria() {
    // Empty criteria should not match
    assert!(!match_criteria_host("", "host", "alice", "host"));
}

#[test]
fn match_criteria_host_only_unknown_keyword() {
    // Only unsupported keywords — user "alice" doesn't match target_user "bob".
    assert!(!match_criteria_host("exec true", "host", "bob", "host"));
}

#[test]
fn simple_glob_match_empty_pattern() {
    // Empty pattern matches empty text
    assert!(simple_glob_match("", ""));
    assert!(!simple_glob_match("a", ""));
}

#[test]
fn simple_glob_match_star_only() {
    assert!(simple_glob_match("", "*"));
    assert!(simple_glob_match("anything", "*"));
}

#[test]
fn simple_glob_match_question_only() {
    // simple_glob_match delegates to glob_matches when pattern contains ?
    // glob_matches handles ? as a single-character wildcard
    assert!(!simple_glob_match("", "?"));
    assert!(simple_glob_match("a", "?"));  // ? matches one char
    assert!(!simple_glob_match("ab", "?")); // ? matches exactly one char
}

#[test]
fn match_criteria_host_negation() {
    // Negation patterns must be comma-separated with positive patterns
    assert!(!match_criteria_host("host *,!badhost", "badhost", "alice", "badhost"));
    assert!(match_criteria_host("host *,!badhost", "goodhost", "alice", "goodhost"));
}

// ---------------------------------------------------------------------------
// Edge case tests for expand_env_vars (v2 — no duplicate names)
// ---------------------------------------------------------------------------

#[test]
fn expand_env_vars_empty_braces() {
    // ${} with empty name — should expand to empty
    assert_eq!(expand_env_vars("${}"), "");
}

#[test]
fn expand_env_vars_trailing_dollar() {
    // Bare $ at end of string
    assert_eq!(expand_env_vars("test$"), "test$");
}

#[test]
fn expand_env_vars_no_braces_in_path() {
    unsafe { std::env::set_var("TORIDE_PVAR", "/usr/local"); }
    assert_eq!(expand_env_vars("$TORIDE_PVAR/bin"), "/usr/local/bin");
    unsafe { std::env::remove_var("TORIDE_PVAR"); }
}

// ---------------------------------------------------------------------------
// Edge case tests for expand_tokens (v2)
// ---------------------------------------------------------------------------

#[test]
fn expand_tokens_unknown_x() {
    let ctx = TokenContext { host: "h", home_dir: "d", local_hostname: "l", remote_user: "r", local_user: "u", port: "p", canonical_host: "h" };
    assert_eq!(expand_tokens("%X", &ctx), "%X");
}

#[test]
fn expand_tokens_double_percent_preserved() {
    let ctx = TokenContext { host: "h", home_dir: "d", local_hostname: "l", remote_user: "r", local_user: "u", port: "p", canonical_host: "h" };
    assert_eq!(expand_tokens("%%", &ctx), "%%");
}

#[test]
fn expand_tokens_empty() {
    let ctx = TokenContext { host: "h", home_dir: "d", local_hostname: "l", remote_user: "r", local_user: "u", port: "p", canonical_host: "h" };
    assert_eq!(expand_tokens("", &ctx), "");
}

// ---------------------------------------------------------------------------
// Edge case tests for collapse_double_percent (v2)
// ---------------------------------------------------------------------------

#[test]
fn collapse_double_pct_basic() {
    assert_eq!(collapse_double_percent("%%"), "%");
    assert_eq!(collapse_double_percent("100%%"), "100%");
    assert_eq!(collapse_double_percent("%%test"), "%test");
}

#[test]
fn collapse_double_pct_none() {
    assert_eq!(collapse_double_percent("test"), "test");
    assert_eq!(collapse_double_percent("%h"), "%h");
}

#[test]
fn collapse_double_pct_empty() {
    assert_eq!(collapse_double_percent(""), "");
}

// ---------------------------------------------------------------------------
// Tests for Match user criterion
// ---------------------------------------------------------------------------

#[test]
fn match_criteria_user_basic() {
    assert!(match_criteria_host("user alice", "host", "alice", "host"));
    assert!(!match_criteria_host("user alice", "host", "bob", "host"));
}

#[test]
fn match_criteria_user_case_insensitive() {
    assert!(match_criteria_host("user Alice", "host", "alice", "host"));
    assert!(match_criteria_host("user ALICE", "host", "alice", "host"));
    assert!(match_criteria_host("user alice", "host", "ALICE", "host"));
}

#[test]
fn match_criteria_user_multiple_names() {
    assert!(match_criteria_host("user alice,bob", "host", "alice", "host"));
    assert!(match_criteria_host("user alice,bob", "host", "bob", "host"));
    assert!(!match_criteria_host("user alice,bob", "host", "charlie", "host"));
}

#[test]
fn match_criteria_user_with_host() {
    // Both user and host must match (AND logic).
    assert!(match_criteria_host(
        "user alice host web",
        "web",
        "alice",
        "web",
    ));
    assert!(!match_criteria_host(
        "user alice host web",
        "web",
        "bob",
        "web",
    ));
    assert!(!match_criteria_host(
        "user alice host web",
        "db",
        "alice",
        "web",
    ));
}

// ---------------------------------------------------------------------------
// Tests for Match originalhost criterion
// ---------------------------------------------------------------------------

#[test]
fn match_criteria_originalhost_basic() {
    assert!(match_criteria_host(
        "originalhost web",
        "canonical.web",
        "alice",
        "web",
    ));
    assert!(!match_criteria_host(
        "originalhost web",
        "canonical.web",
        "alice",
        "db",
    ));
}

#[test]
fn match_criteria_originalhost_wildcard() {
    assert!(match_criteria_host(
        "originalhost *.example.com",
        "canonical.example.com",
        "alice",
        "web.example.com",
    ));
}

#[test]
fn match_criteria_originalhost_with_host() {
    // Both originalhost and host are checked.
    // originalhost matches against the original alias, host against the
    // current target (which may be the canonical name).
    assert!(match_criteria_host(
        "originalhost web host canonical.web",
        "canonical.web",
        "alice",
        "web",
    ));
    assert!(!match_criteria_host(
        "originalhost web host canonical.web",
        "canonical.web",
        "alice",
        "other",
    ));
}

#[test]
fn match_criteria_originalhost_negation() {
    assert!(!match_criteria_host(
        "originalhost *,!badhost",
        "canonical",
        "alice",
        "badhost",
    ));
    assert!(match_criteria_host(
        "originalhost *,!badhost",
        "canonical",
        "alice",
        "goodhost",
    ));
}

// ---------------------------------------------------------------------------
// Tests for CanonicalizeHostname awareness
// ---------------------------------------------------------------------------

#[test]
fn is_canonicalize_enabled_yes() {
    let resolved = ResolvedHost {
        alias: "test".into(),
        host_name: None,
        user: None,
        port: None,
        identity_files: vec![],
        proxy_jump: None,
        directives: vec![("CanonicalizeHostname".into(), "yes".into())],
        canonicalized: false,
    };
    assert!(is_canonicalize_enabled(&resolved));
}

#[test]
fn is_canonicalize_enabled_always() {
    let resolved = ResolvedHost {
        alias: "test".into(),
        host_name: None,
        user: None,
        port: None,
        identity_files: vec![],
        proxy_jump: None,
        directives: vec![("CanonicalizeHostname".into(), "always".into())],
        canonicalized: false,
    };
    assert!(is_canonicalize_enabled(&resolved));
}

#[test]
fn is_canonicalize_enabled_no() {
    let resolved = ResolvedHost {
        alias: "test".into(),
        host_name: None,
        user: None,
        port: None,
        identity_files: vec![],
        proxy_jump: None,
        directives: vec![("CanonicalizeHostname".into(), "no".into())],
        canonicalized: false,
    };
    assert!(!is_canonicalize_enabled(&resolved));
}

#[test]
fn is_canonicalize_enabled_missing() {
    let resolved = ResolvedHost {
        alias: "test".into(),
        host_name: None,
        user: None,
        port: None,
        identity_files: vec![],
        proxy_jump: None,
        directives: vec![],
        canonicalized: false,
    };
    assert!(!is_canonicalize_enabled(&resolved));
}

#[test]
fn is_canonicalize_enabled_case_insensitive() {
    let resolved = ResolvedHost {
        alias: "test".into(),
        host_name: None,
        user: None,
        port: None,
        identity_files: vec![],
        proxy_jump: None,
        directives: vec![("canonicalizehostname".into(), "YES".into())],
        canonicalized: false,
    };
    assert!(is_canonicalize_enabled(&resolved));
}

// ---------------------------------------------------------------------------
// Tests for expanded token expansion (additional directives)
// ---------------------------------------------------------------------------

#[test]
fn expand_resolved_certificate_file() {
    let mut resolved = ResolvedHost {
        alias: "h".into(),
        host_name: None,
        user: None,
        port: None,
        identity_files: vec![],
        proxy_jump: None,
        directives: vec![("CertificateFile".into(), "%d/.ssh/%h-cert.pub".into())],
        canonicalized: false,
    };
    expand_resolved(&mut resolved, "example", Path::new("/tmp"));
    let val = &resolved.directives[0].1;
    // %d and %h should have been expanded.
    assert!(!val.contains("%d"));
    assert!(!val.contains("%h"));
    assert!(val.contains("example-cert.pub"));
}

#[test]
fn expand_resolved_control_path() {
    let mut resolved = ResolvedHost {
        alias: "h".into(),
        host_name: None,
        user: None,
        port: None,
        identity_files: vec![],
        proxy_jump: None,
        directives: vec![("ControlPath".into(), "/tmp/ssh-%h-%p".into())],
        canonicalized: false,
    };
    expand_resolved(&mut resolved, "example", Path::new("/tmp"));
    let val = &resolved.directives[0].1;
    assert!(!val.contains("%h"));
    assert!(val.contains("example"));
}

#[test]
fn expand_resolved_user_known_hosts_file() {
    let mut resolved = ResolvedHost {
        alias: "h".into(),
        host_name: None,
        user: None,
        port: None,
        identity_files: vec![],
        proxy_jump: None,
        directives: vec![(
            "UserKnownHostsFile".into(),
            "%d/.ssh/known_hosts_%h".into(),
        )],
        canonicalized: false,
    };
    expand_resolved(&mut resolved, "myhost", Path::new("/tmp"));
    let val = &resolved.directives[0].1;
    assert!(!val.contains("%d"));
    assert!(!val.contains("%h"));
    assert!(val.contains("myhost"));
}

#[test]
fn expand_resolved_identity_agent() {
    let mut resolved = ResolvedHost {
        alias: "h".into(),
        host_name: None,
        user: None,
        port: None,
        identity_files: vec![],
        proxy_jump: None,
        directives: vec![(
            "IdentityAgent".into(),
            "${SSH_AUTH_SOCK}".into(),
        )],
        canonicalized: false,
    };
    expand_resolved(&mut resolved, "h", Path::new("/tmp"));
    // The env var should have been expanded (or empty if not set).
    // The important thing is it doesn't contain the literal ${...}.
    let val = &resolved.directives[0].1;
    assert!(!val.contains("${SSH_AUTH_SOCK}"));
}

// ---------------------------------------------------------------------------
// Tests for resolve_pass (internal)
// ---------------------------------------------------------------------------

#[test]
fn resolve_pass_default_canonicalized_false() {
    // A resolved host from resolve_pass should have canonicalized = false.
    use super::ast;
    let ast = ast::parse("Host example\n  HostName example.com\n");
    let resolved = resolve_pass(&ast, "example", "example", "user", Path::new("/tmp"));
    assert!(!resolved.canonicalized);
    assert_eq!(resolved.host_name.as_deref(), Some("example.com"));
}

// ---------------------------------------------------------------------------
// Edge case: Include chain with cycle detection
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resolve_include_cycle_detected() {
    // Create a config that includes itself through a chain: config -> a -> b -> a
    let dir = tempfile::tempdir().unwrap();
    let ssh_dir = dir.path();

    // config includes chain_a
    std::fs::write(
        ssh_dir.join("config"),
        "Include chain_a\nHost main\n    User root\n",
    )
    .unwrap();

    // chain_a includes chain_b
    std::fs::write(
        ssh_dir.join("chain_a"),
        "Host alpha\n    User alice\nInclude chain_b\n",
    )
    .unwrap();

    // chain_b includes chain_a (creates cycle!)
    std::fs::write(
        ssh_dir.join("chain_b"),
        "Host beta\n    User bob\nInclude chain_a\n",
    )
    .unwrap();

    let result = resolve(ssh_dir, "alpha", None).await;

    assert!(result.is_err(), "should detect include cycle");
    match result.unwrap_err() {
        crate::Error::ConfigIncludeCycle(path) => {
            assert!(
                path.contains("chain_a"),
                "cycle error should mention the offending file, got: {path}"
            );
        }
        other => panic!("expected ConfigIncludeCycle error, got: {other}"),
    }
}

#[tokio::test]
async fn resolve_include_self_referencing() {
    // A config that directly includes itself.
    let dir = tempfile::tempdir().unwrap();
    let ssh_dir = dir.path();

    std::fs::write(
        ssh_dir.join("config"),
        "Include config\nHost self\n    User me\n",
    )
    .unwrap();

    let result = resolve(ssh_dir, "self", None).await;

    assert!(result.is_err(), "should detect self-referencing include");
    match result.unwrap_err() {
        crate::Error::ConfigIncludeCycle(_) => {}
        other => panic!("expected ConfigIncludeCycle error, got: {other}"),
    }
}

#[tokio::test]
async fn resolve_include_chain_without_cycle() {
    // A valid include chain with no cycle: config -> layer1 -> layer2
    let dir = tempfile::tempdir().unwrap();
    let ssh_dir = dir.path();

    std::fs::write(
        ssh_dir.join("config"),
        "Include layer1\nHost main\n    User root\n",
    )
    .unwrap();

    std::fs::write(
        ssh_dir.join("layer1"),
        "Include layer2\nHost web\n    User deploy\n",
    )
    .unwrap();

    std::fs::write(
        ssh_dir.join("layer2"),
        "Host db\n    User admin\n",
    )
    .unwrap();

    let resolved = resolve(ssh_dir, "db", None).await;
    assert!(resolved.is_ok(), "valid include chain should not error");
    let resolved = resolved.unwrap();
    assert_eq!(resolved.user.as_deref(), Some("admin"));
}

#[tokio::test]
async fn resolve_include_nonexistent_file() {
    // Including a file that does not exist should not error (OpenSSH behavior).
    let dir = tempfile::tempdir().unwrap();
    let ssh_dir = dir.path();

    std::fs::write(
        ssh_dir.join("config"),
        "Include does_not_exist\nHost test\n    User alice\n",
    )
    .unwrap();

    let resolved = resolve(ssh_dir, "test", None).await;
    assert!(resolved.is_ok(), "missing include file should be silently skipped");
    assert_eq!(resolved.unwrap().user.as_deref(), Some("alice"));
}
