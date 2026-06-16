//! Editing of `/etc/ssh/sshd_config` for access-control directives.
//!
//! This reuses the lossless [`ast::ConfigAst`] (which already handles `Match`
//! blocks, `Include`, comments, and whitespace faithfully) to provide focused
//! read/edit operations for the user-access directives Toride cares about:
//! `AllowUsers` and `DenyUsers`.
//!
//! Writes are privileged (the file is root-owned) and go through
//! [`toride_ssh_core::run_privileged`], which validates the result with
//! `sshd -t` and keeps a `.bak` backup before installing — a malformed
//! sshd_config would lock the user out of SSH, so an invalid config is never
//! written.
//!
//! All access here is restricted to **global** directives (those outside any
//! `Match` block). `Match`-scoped directives are intentionally left untouched
//! — editing them blindly is dangerous, and global scope is what we expose in
//! the UI.

use toride_ssh_core::{privilege::PrivilegedOp, run_privileged, Result};

use super::ast::{parse, ConfigAst, ConfigNode, DirectiveData, Separator};

/// Path to the system SSH daemon configuration.
const SSHD_CONFIG_PATH: &str = "/etc/ssh/sshd_config";

// ---------------------------------------------------------------------------
// Load / save / edit
// ---------------------------------------------------------------------------

/// Load and parse `/etc/ssh/sshd_config` into a lossless AST.
///
/// Returns an empty AST if the file does not exist.
///
/// # Errors
///
/// Returns [`Error::Io`] if the file exists but cannot be read.
pub async fn load() -> Result<ConfigAst> {
    let path = std::path::Path::new(SSHD_CONFIG_PATH);
    if !path.exists() {
        return Ok(ConfigAst { nodes: Vec::new() });
    }
    let content = tokio::fs::read_to_string(path).await?;
    Ok(parse(&content))
}

/// Serialize the AST and install it as `/etc/ssh/sshd_config` via a privileged
/// operation (validation + backup + atomic install).
///
/// `running_as_root` selects direct writes vs. the `sudo -n` path.
///
/// # Errors
///
/// Returns [`Error::SshdConfigInvalid`] if `sshd -t` rejects the config, or
/// [`Error::SudoFailed`] if the privileged install can't run.
pub async fn save(ast: &ConfigAst, running_as_root: bool) -> Result<()> {
    let content = ast.to_string_lossless();
    run_privileged(PrivilegedOp::WriteSshdConfig { content }, running_as_root).await
}

/// Load, mutate, and save `/etc/ssh/sshd_config` in one call.
///
/// Mirrors [`ConfigService::edit`](super::ConfigService::edit): the closure
/// mutates the AST in place; on success the result is validated and installed.
///
/// # Errors
///
/// Propagates any error from loading, the closure, or saving.
pub async fn edit<F>(running_as_root: bool, f: F) -> Result<()>
where
    F: FnOnce(&mut ConfigAst) -> Result<()>,
{
    let mut ast = load().await?;
    f(&mut ast)?;
    save(&ast, running_as_root).await
}

// ---------------------------------------------------------------------------
// Read
// ---------------------------------------------------------------------------

/// The current global `AllowUsers` list (ignoring `Match`-scoped directives).
///
/// OpenSSH treats multiple `AllowUsers` lines as additive, so values from all
/// global occurrences are concatenated.
pub fn get_allow_users(ast: &ConfigAst) -> Vec<String> {
    collect_global_users(ast, "AllowUsers")
}

/// The current global `DenyUsers` list (ignoring `Match`-scoped directives).
pub fn get_deny_users(ast: &ConfigAst) -> Vec<String> {
    collect_global_users(ast, "DenyUsers")
}

/// The current global `AllowGroups` list (ignoring `Match`-scoped directives).
///
/// OpenSSH treats multiple `AllowGroups` lines as additive, so values from all
/// global occurrences are concatenated.
pub fn get_allow_groups(ast: &ConfigAst) -> Vec<String> {
    collect_global_users(ast, "AllowGroups")
}

/// The current global `DenyGroups` list (ignoring `Match`-scoped directives).
pub fn get_deny_groups(ast: &ConfigAst) -> Vec<String> {
    collect_global_users(ast, "DenyGroups")
}

/// Collect every whitespace-separated token from all global `<key>` directives.
///
/// Only top-level [`ConfigNode::Directive`] nodes are considered — values
/// inside `Match`/`Host` blocks are intentionally skipped, since those are
/// scope-specific and not what Toride's global UI exposes.
fn collect_global_users(ast: &ConfigAst, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    for node in &ast.nodes {
        if let ConfigNode::Directive(d) = node {
            if d.keyword.eq_ignore_ascii_case(key) {
                out.extend(d.value.split_whitespace().map(str::to_owned));
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Pattern detection
// ---------------------------------------------------------------------------

/// Returns `true` if `value` contains any OpenSSH pattern tokens.
///
/// `AllowUsers`/`DenyUsers` values are pattern strings: a token may contain
/// glob wildcards (`*`, `?`) or a `user@host` selector. Such a directive
/// cannot be safely edited by exact username — adding a plain name would be
/// swallowed by an existing pattern, and "removing" against patterns would be
/// semantically meaningless. The editor refuses to mutate such directives
/// instead (see [`directive_has_patterns`]).
///
/// Each whitespace-separated token is inspected independently.
pub fn has_pattern_tokens(value: &str) -> bool {
    value
        .split_whitespace()
        .any(|tok| tok.contains('*') || tok.contains('?') || tok.contains('@'))
}

/// Returns `true` if any global `<key>` directive contains pattern tokens.
///
/// `Match`-scoped directives are ignored, consistent with the rest of this
/// module's global-only scope.
pub fn directive_has_patterns(ast: &ConfigAst, key: &str) -> bool {
    ast.nodes.iter().any(|node| match node {
        ConfigNode::Directive(d) if d.keyword.eq_ignore_ascii_case(key) => {
            has_pattern_tokens(&d.value)
        }
        _ => false,
    })
}

// ---------------------------------------------------------------------------
// Write
// ---------------------------------------------------------------------------

/// Add `user` to the global `AllowUsers` directive.
///
/// Idempotent: a no-op if the user is already listed. If no `AllowUsers`
/// directive exists, one is appended at the end of the global section (before
/// the first `Match`/`Host` block, or at the end of the file).
///
/// Multiple global `AllowUsers` lines are consolidated into the first
/// occurrence (OpenSSH treats them as additive, so the merge is semantically
/// equivalent).
///
/// # Errors
///
/// Returns [`Error::SshdConfigInvalid`] if an existing `AllowUsers` directive
/// uses pattern tokens (`*`, `?`, `@`); the AST is left unmodified in that
/// case. Editing a pattern directive by exact username would be semantically
/// wrong.
pub fn add_user_to_allow(ast: &mut ConfigAst, user: &str) -> Result<()> {
    upsert_user_in_directive(ast, "AllowUsers", user, Action::Add)
}

/// Remove `user` from the global `AllowUsers` directive.
///
/// If the removal empties the list, the directive is deleted entirely (an
/// absent `AllowUsers` means "all allowed", which is the OpenSSH default and
/// clearer than a dangling empty line). Multiple global `AllowUsers` lines are
/// merged first, so removing a user who lived only on a later line succeeds.
///
/// # Errors
///
/// Returns [`Error::SshdConfigInvalid`] if an existing `AllowUsers` directive
/// uses pattern tokens (`*`, `?`, `@`); the AST is left unmodified.
pub fn remove_user_from_allow(ast: &mut ConfigAst, user: &str) -> Result<()> {
    upsert_user_in_directive(ast, "AllowUsers", user, Action::Remove)
}

/// Add `user` to the global `DenyUsers` directive. Idempotent.
///
/// # Errors
///
/// Returns [`Error::SshdConfigInvalid`] if an existing `DenyUsers` directive
/// uses pattern tokens (`*`, `?`, `@`); the AST is left unmodified.
pub fn add_user_to_deny(ast: &mut ConfigAst, user: &str) -> Result<()> {
    upsert_user_in_directive(ast, "DenyUsers", user, Action::Add)
}

/// Remove `user` from the global `DenyUsers` directive, deleting the directive
/// if it becomes empty.
///
/// # Errors
///
/// Returns [`Error::SshdConfigInvalid`] if an existing `DenyUsers` directive
/// uses pattern tokens (`*`, `?`, `@`); the AST is left unmodified.
pub fn remove_user_from_deny(ast: &mut ConfigAst, user: &str) -> Result<()> {
    upsert_user_in_directive(ast, "DenyUsers", user, Action::Remove)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Add,
    Remove,
}

/// Add or remove `user` from the global `<key>` directive(s), creating or
/// deleting the directive as needed.
///
/// Refuses to touch the AST (returning [`Error::SshdConfigInvalid`]) if any
/// global occurrence of `<key>` contains pattern tokens — editing a pattern
/// directive by exact username would be semantically wrong.
///
/// When multiple global occurrences exist, they are merged into the first
/// (deduplicated, order-preserving) and the rest are deleted. OpenSSH treats
/// multiple lines as additive, so the merge is lossless from the daemon's
/// perspective and keeps the file readable. A warning is logged when a merge
/// occurs.
fn upsert_user_in_directive(
    ast: &mut ConfigAst,
    key: &str,
    user: &str,
    action: Action,
) -> Result<()> {
    let indices: Vec<usize> = ast
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| {
            matches!(n, ConfigNode::Directive(d) if d.keyword.eq_ignore_ascii_case(key))
        })
        .map(|(i, _)| i)
        .collect();

    // Refuse to mutate any directive that already uses patterns — exact
    // username edits against glob/@ selectors are semantically wrong. Check
    // before touching anything so the AST is left unmodified on error.
    if indices
        .iter()
        .any(|&i| matches!(&ast.nodes[i], ConfigNode::Directive(d) if has_pattern_tokens(&d.value)))
    {
        return Err(toride_ssh_core::Error::SshdConfigInvalid(format!(
            "{key} uses pattern tokens (* ? @); refusing to edit by exact username"
        )));
    }

    if indices.len() > 1 {
        tracing::warn!(
            "{key} appears {} times in sshd_config; merging into the first occurrence",
            indices.len()
        );
    }

    match indices.first() {
        Some(&first) => {
            // Gather the UNION of values across all global occurrences
            // (deduplicated, order-preserving), then drop occurrences 2..N.
            let mut merged: Vec<String> = Vec::new();
            for &i in &indices {
                if let ConfigNode::Directive(d) = &ast.nodes[i] {
                    for tok in d.value.split_whitespace() {
                        if !merged.iter().any(|m| m == tok) {
                            merged.push(tok.to_owned());
                        }
                    }
                }
            }

            // Apply the requested Add/Remove against the merged union.
            apply_action_to_users(&mut merged, user, action);

            // Write the result back into the first occurrence (or delete it if
            // the union is now empty — an absent directive is clearer than a
            // dangling empty line and matches the OpenSSH default).
            if merged.is_empty() {
                // Remove ALL occurrences (first first, then the rest in reverse
                // so earlier indices stay valid as we drain).
                for &i in indices.iter().rev() {
                    ast.nodes.remove(i);
                }
            } else {
                let joined = merged.join(" ");
                if let ConfigNode::Directive(d) = &mut ast.nodes[first] {
                    d.value = joined;
                }
                // Delete occurrences 2..N. Iterate in reverse to keep earlier
                // indices valid as we remove.
                for &i in indices.iter().skip(1).rev() {
                    ast.nodes.remove(i);
                }
            }
        }
        // No directive yet: add one (only on Add; Remove of a missing user is
        // a harmless no-op).
        None => {
            if action == Action::Add {
                append_global_directive(ast, key, user);
            }
        }
    }

    Ok(())
}

/// Apply an Add/Remove to an in-memory user list (order-preserving).
fn apply_action_to_users(users: &mut Vec<String>, user: &str, action: Action) {
    match action {
        Action::Add => {
            if !users.iter().any(|u| u == user) {
                users.push(user.to_owned());
            }
        }
        Action::Remove => {
            users.retain(|u| u != user);
        }
    }
}

/// Append a new global `<key> <value>` directive, placed before the first
/// `Match`/`Host` block (or at the end if there are none).
fn append_global_directive(ast: &mut ConfigAst, key: &str, value: &str) {
    let directive = ConfigNode::Directive(Box::new(DirectiveData {
        keyword: key.to_owned(),
        separator: Separator::Space,
        value: value.to_owned(),
        comment: None,
        indent: String::new(),
    }));

    let insert_pos = ast
        .nodes
        .iter()
        .position(|n| matches!(n, ConfigNode::HostBlock(_) | ConfigNode::MatchBlock(_)))
        .unwrap_or(ast.nodes.len());

    ast.nodes.insert(insert_pos, directive);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ast(input: &str) -> ConfigAst {
        parse(input)
    }

    #[test]
    fn get_allow_users_reads_global_directive() {
        let a = ast("Port 22\nAllowUsers alice bob\n");
        assert_eq!(get_allow_users(&a), vec!["alice", "bob"]);
    }

    #[test]
    fn get_allow_users_ignores_match_scoped() {
        let a = ast("AllowUsers alice\nMatch User carol\n    AllowUsers bob\n");
        // Only the global alice; the Match-scoped bob is ignored.
        assert_eq!(get_allow_users(&a), vec!["alice"]);
    }

    #[test]
    fn get_deny_users_is_empty_when_absent() {
        let a = ast("Port 22\n");
        assert!(get_deny_users(&a).is_empty());
    }

    #[test]
    fn add_user_to_allow_creates_directive() {
        let mut a = ast("Port 22\n");
        add_user_to_allow(&mut a, "alice").unwrap();
        assert_eq!(get_allow_users(&a), vec!["alice"]);
    }

    #[test]
    fn add_user_to_allow_is_idempotent() {
        let mut a = ast("AllowUsers alice\n");
        add_user_to_allow(&mut a, "alice").unwrap();
        assert_eq!(get_allow_users(&a), vec!["alice"]);
    }

    #[test]
    fn add_user_to_allow_appends_to_existing() {
        let mut a = ast("AllowUsers alice\n");
        add_user_to_allow(&mut a, "bob").unwrap();
        assert_eq!(get_allow_users(&a), vec!["alice", "bob"]);
    }

    #[test]
    fn remove_user_from_allow_deletes_directive_when_empty() {
        let mut a = ast("AllowUsers alice\n");
        remove_user_from_allow(&mut a, "alice").unwrap();
        // Directive should be gone entirely.
        assert!(get_allow_users(&a).is_empty());
        assert!(!a.to_string_lossless().contains("AllowUsers"));
    }

    #[test]
    fn remove_user_from_allow_keeps_others() {
        let mut a = ast("AllowUsers alice bob\n");
        remove_user_from_allow(&mut a, "alice").unwrap();
        assert_eq!(get_allow_users(&a), vec!["bob"]);
    }

    #[test]
    fn remove_missing_user_is_noop() {
        let mut a = ast("AllowUsers alice\n");
        remove_user_from_allow(&mut a, "zzz").unwrap();
        assert_eq!(get_allow_users(&a), vec!["alice"]);
    }

    #[test]
    fn add_deny_then_reset_removes_from_both() {
        let mut a = ast("AllowUsers alice\n");
        add_user_to_deny(&mut a, "alice").unwrap();
        // alice is now in both lists; reset = remove from allow and deny.
        remove_user_from_allow(&mut a, "alice").unwrap();
        remove_user_from_deny(&mut a, "alice").unwrap();
        assert!(get_allow_users(&a).is_empty());
        assert!(get_deny_users(&a).is_empty());
    }

    #[test]
    fn round_trip_preserves_comments_and_match_blocks() {
        let input = "# top comment\nPort 22\n\nMatch User alice\n    PermitRootLogin no\n";
        let mut a = ast(input);
        add_user_to_allow(&mut a, "bob").unwrap();
        let out = a.to_string_lossless();
        assert!(out.contains("# top comment"), "comment preserved");
        assert!(out.contains("Match User alice"), "match block preserved");
        assert!(out.contains("PermitRootLogin no"), "match body preserved");
        assert!(out.contains("AllowUsers bob"));
    }

    #[test]
    fn new_directive_inserted_before_match_block() {
        let mut a = ast("Port 22\nMatch User alice\n    X11Forwarding no\n");
        add_user_to_allow(&mut a, "bob").unwrap();
        let out = a.to_string_lossless();
        let allow_pos = out.find("AllowUsers").unwrap();
        let match_pos = out.find("Match User").unwrap();
        assert!(allow_pos < match_pos, "AllowUsers must precede the Match block");
    }

    // --- pattern-token detection ------------------------------------------

    #[test]
    fn has_pattern_tokens_detects_wildcards() {
        assert!(has_pattern_tokens("alice * bob"));
        assert!(has_pattern_tokens("ali?ce"));
        assert!(has_pattern_tokens("alice@host"));
        assert!(!has_pattern_tokens("alice bob carol"));
        assert!(!has_pattern_tokens(""));
    }

    #[test]
    fn directive_has_patterns_scans_global_only() {
        // Global wildcard occurrence.
        let a = ast("AllowUsers *\n");
        assert!(directive_has_patterns(&a, "AllowUsers"));

        // Plain usernames.
        let a = ast("AllowUsers alice bob\n");
        assert!(!directive_has_patterns(&a, "AllowUsers"));

        // Pattern only inside a Match block is ignored (global scope).
        let a = ast("AllowUsers alice\nMatch User carol\n    AllowUsers *\n");
        assert!(
            !directive_has_patterns(&a, "AllowUsers"),
            "Match-scoped patterns must not count"
        );
    }

    #[test]
    fn add_user_to_allow_refuses_pattern_directive() {
        let mut a = ast("AllowUsers *\n");
        let before = a.to_string_lossless();
        let err = add_user_to_allow(&mut a, "bob").unwrap_err();
        assert!(matches!(
            err,
            toride_ssh_core::Error::SshdConfigInvalid(_)
        ));
        // AST must be untouched.
        assert_eq!(a.to_string_lossless(), before);
    }

    #[test]
    fn remove_user_from_allow_refuses_pattern_directive() {
        let mut a = ast("AllowUsers alice *@host\n");
        let before = a.to_string_lossless();
        let err = remove_user_from_allow(&mut a, "alice").unwrap_err();
        assert!(matches!(
            err,
            toride_ssh_core::Error::SshdConfigInvalid(_)
        ));
        assert_eq!(a.to_string_lossless(), before);
    }

    #[test]
    fn add_user_to_deny_refuses_question_pattern() {
        let mut a = ast("DenyUsers ?uest\n");
        let before = a.to_string_lossless();
        let err = add_user_to_deny(&mut a, "bob").unwrap_err();
        assert!(matches!(
            err,
            toride_ssh_core::Error::SshdConfigInvalid(_)
        ));
        assert_eq!(a.to_string_lossless(), before);
    }

    // --- multi-line consolidation -----------------------------------------

    #[test]
    fn add_merges_multiple_allow_users_lines() {
        let mut a = ast("AllowUsers alice\nPort 22\nAllowUsers bob\n");
        add_user_to_allow(&mut a, "carol").unwrap();
        // Union on a single line; second occurrence gone.
        assert_eq!(get_allow_users(&a), vec!["alice", "bob", "carol"]);
        assert_eq!(
            a.to_string_lossless()
                .matches("AllowUsers")
                .count(),
            1,
            "exactly one AllowUsers line after merge"
        );
    }

    #[test]
    fn add_merge_dedupes_and_preserves_order() {
        let mut a = ast("AllowUsers alice bob\nAllowUsers bob carol\n");
        add_user_to_allow(&mut a, "alice").unwrap();
        assert_eq!(get_allow_users(&a), vec!["alice", "bob", "carol"]);
        assert_eq!(
            a.to_string_lossless().matches("AllowUsers").count(),
            1
        );
    }

    #[test]
    fn remove_across_multiple_occurrences() {
        // bob lives only on the second line; removal must still succeed.
        let mut a = ast("AllowUsers alice\nAllowUsers bob\n");
        remove_user_from_allow(&mut a, "bob").unwrap();
        assert_eq!(get_allow_users(&a), vec!["alice"]);
        assert_eq!(
            a.to_string_lossless().matches("AllowUsers").count(),
            1
        );
    }

    #[test]
    fn remove_empties_union_deletes_all_occurrences() {
        // alice spread across two lines; removing her empties the union.
        let mut a = ast("AllowUsers alice\nAllowUsers alice\n");
        remove_user_from_allow(&mut a, "alice").unwrap();
        assert!(get_allow_users(&a).is_empty());
        assert!(
            !a.to_string_lossless().contains("AllowUsers"),
            "directive deleted entirely when union is empty"
        );
    }

    // --- group getters + read-path coverage -------------------------------

    #[test]
    fn get_allow_groups_reads_global() {
        let a = ast("AllowGroups wheel staff\nMatch User carol\n    AllowGroups extra\n");
        assert_eq!(get_allow_groups(&a), vec!["wheel", "staff"]);
    }

    #[test]
    fn get_deny_groups_reads_global_and_skips_match() {
        let a = ast("DenyGroups banned\nMatch User carol\n    DenyGroups scoped\n");
        assert_eq!(get_deny_groups(&a), vec!["banned"]);
    }

    #[test]
    fn get_deny_users_concatenates_multiple_lines() {
        let a = ast("DenyUsers alice\nDenyUsers bob\n");
        assert_eq!(get_deny_users(&a), vec!["alice", "bob"]);
    }

    #[test]
    fn get_groups_empty_when_absent() {
        let a = ast("Port 22\n");
        assert!(get_allow_groups(&a).is_empty());
        assert!(get_deny_groups(&a).is_empty());
    }
}
