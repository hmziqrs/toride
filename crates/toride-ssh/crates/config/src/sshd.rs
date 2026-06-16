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

/// Collect every whitespace-separated token from all global `<key>` directives.
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
// Write
// ---------------------------------------------------------------------------

/// Add `user` to the global `AllowUsers` directive.
///
/// Idempotent: a no-op if the user is already listed. If no `AllowUsers`
/// directive exists, one is appended at the end of the global section (before
/// the first `Match`/`Host` block, or at the end of the file).
///
/// # Errors
///
/// Currently always returns `Ok`; reserved for future validation.
pub fn add_user_to_allow(ast: &mut ConfigAst, user: &str) -> Result<()> {
    upsert_user_in_directive(ast, "AllowUsers", user, Action::Add)
}

/// Remove `user` from the global `AllowUsers` directive.
///
/// If the removal empties the list, the directive is deleted entirely (an
/// absent `AllowUsers` means "all allowed", which is the OpenSSH default and
/// clearer than a dangling empty line).
///
/// # Errors
///
/// Currently always returns `Ok`; reserved for future validation.
pub fn remove_user_from_allow(ast: &mut ConfigAst, user: &str) -> Result<()> {
    upsert_user_in_directive(ast, "AllowUsers", user, Action::Remove)
}

/// Add `user` to the global `DenyUsers` directive. Idempotent.
///
/// # Errors
///
/// Currently always returns `Ok`; reserved for future validation.
pub fn add_user_to_deny(ast: &mut ConfigAst, user: &str) -> Result<()> {
    upsert_user_in_directive(ast, "DenyUsers", user, Action::Add)
}

/// Remove `user` from the global `DenyUsers` directive, deleting the directive
/// if it becomes empty.
///
/// # Errors
///
/// Currently always returns `Ok`; reserved for future validation.
pub fn remove_user_from_deny(ast: &mut ConfigAst, user: &str) -> Result<()> {
    upsert_user_in_directive(ast, "DenyUsers", user, Action::Remove)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Add,
    Remove,
}

/// Add or remove `user` from the first global `<key>` directive, creating or
/// deleting the directive as needed. Consolidates into the first occurrence
/// and warns if multiple occurrences exist (Phase 3 will merge them fully).
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

    if indices.len() > 1 {
        tracing::warn!(
            "{key} appears {} times in sshd_config; consolidating into the first occurrence",
            indices.len()
        );
    }

    match indices.first() {
        // Modify the first global occurrence in place.
        Some(&idx) => {
            if let ConfigNode::Directive(d) = &mut ast.nodes[idx] {
                apply_to_value(&mut d.value, user, action);
                // If the directive is now empty, remove it entirely.
                if d.value.trim().is_empty() {
                    ast.nodes.remove(idx);
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

/// Apply an Add/Remove to a directive's whitespace-separated `value`.
fn apply_to_value(value: &mut String, user: &str, action: Action) {
    let mut users: Vec<String> = value.split_whitespace().map(str::to_owned).collect();
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
    *value = users.join(" ");
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
}
