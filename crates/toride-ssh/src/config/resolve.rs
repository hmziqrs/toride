//! Full SSH config resolution.
//!
//! Handles Include chains, token/env expansion, first-match-wins
//! (with IdentityFile accumulation), and CanonicalizeHostname double-parse.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::ast::{self, ConfigAst, ConfigNode};
use crate::Result;

/// Fully resolved parameters for a host.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolvedHost {
    /// The alias used to look up the host.
    pub alias: String,
    /// The real hostname to connect to.
    pub host_name: Option<String>,
    /// The user for the SSH connection.
    pub user: Option<String>,
    /// The port number.
    pub port: Option<u16>,
    /// Identity files to try.
    pub identity_files: Vec<String>,
    /// ProxyJump hosts.
    pub proxy_jump: Option<String>,
    /// All raw key-value directives from matching blocks.
    pub directives: Vec<(String, String)>,
    /// Whether the config was re-resolved after `CanonicalizeHostname` took
    /// effect. When `true`, `%H` tokens expand to the canonical hostname
    /// rather than the original alias.
    pub canonicalized: bool,
}

/// Directives whose values may contain SSH tokens (`%h`, `%d`, etc.) or
/// tilde/env expansion and should be expanded during resolution.
const TOKEN_EXPANDABLE: &[&str] = &[
    "certificatefile",
    "controlpath",
    "identityagent",
    "localforward",
    "remoteforward",
    "userknownhostsfile",
    "proxycommand",
    "forwardagent",
    "dynamicforward",
    "bindaddress",
    "syslogfacility",
    "kbdinteractiveauthentication",
    "preferredauthentications",
];

/// Fully resolve config for a given host alias.
///
/// This performs:
/// 1. Loading and parsing the main config file.
/// 2. Inlining `Include` directives (with cycle detection).
/// 3. Token and environment variable expansion.
/// 4. First-match-wins resolution with IdentityFile accumulation.
/// 5. If `CanonicalizeHostname` is enabled, a second resolution pass using
///    the resolved `HostName` as the lookup key.
///
/// `user` is the remote username for `Match user` criteria.  When `None`,
/// the local username is used (matching OpenSSH behaviour when no `-l`
/// flag is given).
pub async fn resolve(ssh_dir: &Path, host: &str, user: Option<&str>) -> Result<ResolvedHost> {
    let config_path = ssh_dir.join("config");

    // Load and flatten includes.
    let mut visited = HashSet::new();
    let flat_ast = load_and_flatten(&config_path, &mut visited).await?;

    // First pass: resolve against the original alias.
    let local_user = user.map_or_else(whoami, str::to_owned);
    let mut resolved = resolve_pass(&flat_ast, host, host, &local_user, ssh_dir);

    // Token expansion on first-pass values.
    expand_resolved(&mut resolved, host, ssh_dir);

    // CanonicalizeHostname: if enabled, re-resolve using the resolved HostName.
    if is_canonicalize_enabled(&resolved) {
        let canonical_host = resolved
            .host_name
            .clone()
            .unwrap_or_else(|| host.to_owned());

        let mut canon = resolve_pass(&flat_ast, &canonical_host, host, &local_user, ssh_dir);

        // Expand tokens in the canonicalized result.
        expand_resolved(&mut canon, &canonical_host, ssh_dir);

        host.clone_into(&mut canon.alias);
        canon.canonicalized = true;
        return Ok(canon);
    }

    Ok(resolved)
}

/// Perform a single resolution pass over the flattened AST.
///
/// `target_host` is the hostname used for pattern matching (the canonical
/// name on the second pass, or the original alias on the first).
/// `original_host` is always the alias the user typed — used for
/// `Match originalhost` criteria.
fn resolve_pass(
    flat_ast: &ConfigAst,
    target_host: &str,
    original_host: &str,
    local_user: &str,
    ssh_dir: &Path,
) -> ResolvedHost {
    let mut resolved = ResolvedHost {
        alias: target_host.to_owned(),
        host_name: None,
        user: None,
        port: None,
        identity_files: Vec::new(),
        proxy_jump: None,
        directives: Vec::new(),
        canonicalized: false,
    };

    let mut seen_keys = HashSet::new();

    for node in &flat_ast.nodes {
        match node {
            ConfigNode::HostBlock { patterns, nodes, .. } => {
                if host_matches(target_host, patterns) {
                    resolve_block(
                        nodes,
                        target_host,
                        ssh_dir,
                        &mut resolved,
                        &mut seen_keys,
                    );
                }
            }
            ConfigNode::MatchBlock { criteria, nodes, .. }
                if match_criteria_host(criteria, target_host, local_user, original_host) =>
            {
                resolve_block(
                    nodes,
                    target_host,
                    ssh_dir,
                    &mut resolved,
                    &mut seen_keys,
                );
            }
            _ => {}
        }
    }

    resolved
}

/// Load a config file and recursively inline all Include directives.
fn load_and_flatten<'a>(
    path: &'a Path,
    visited: &'a mut HashSet<PathBuf>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ConfigAst>> + 'a>> {
    Box::pin(async move {
        // PERF: If canonicalize fails (e.g. permission denied, broken symlink),
        // cycle detection may not catch symlink loops. This is acceptable because
        // SSH config files are unlikely to use symlinks.
        let canonical = path
            .canonicalize()
            .unwrap_or_else(|_| path.to_owned());

        if !visited.insert(canonical.clone()) {
            return Err(crate::Error::ConfigIncludeCycle(canonical.display().to_string()));
        }

        let content = if path.exists() {
            tokio::fs::read_to_string(path).await?
        } else {
            return Ok(ConfigAst { nodes: Vec::new() });
        };

        let mut flat = ast::parse(&content);

        // Inline includes.
        let include_nodes: Vec<String> = flat
            .nodes
            .iter()
            .filter_map(|node| {
                if let ConfigNode::Directive { keyword, value, .. } = node
                    && keyword.eq_ignore_ascii_case("include")
                {
                    Some(value.clone())
                } else {
                    None
                }
            })
            .collect();

        for include_pattern in include_nodes {
            let expanded = expand_tilde_and_env(&include_pattern);

            // Glob the pattern.
            let base_dir = if Path::new(&expanded).is_absolute() {
                PathBuf::new()
            } else {
                path.parent().unwrap_or(Path::new(".")).to_owned()
            };

            let full_pattern = base_dir.join(&expanded);
            let pattern_str = full_pattern.display().to_string();

            let matched_files = glob_paths(&pattern_str);

            for inc_path in matched_files {
                let included = load_and_flatten(&inc_path, visited).await?;
                // Insert included nodes in place of the Include directive.
                insert_included_nodes(&mut flat, &included);
            }
        }

    // Remove Include directives after processing.
    flat.nodes
        .retain(|node| !matches!(node, ConfigNode::Directive { keyword, .. } if keyword.eq_ignore_ascii_case("include")));

    Ok(flat)
    })
}

/// Insert included AST nodes in place of the Include directive.
fn insert_included_nodes(flat: &mut ConfigAst, included: &ConfigAst) {
    // Find the first Include directive and replace it with the included nodes.
    let include_idx = flat.nodes.iter().position(|node| {
        matches!(node, ConfigNode::Directive { keyword, .. } if keyword.eq_ignore_ascii_case("include"))
    });

    if let Some(idx) = include_idx {
        flat.nodes.remove(idx);
        flat.nodes.splice(
            idx..idx,
            included.nodes.iter().cloned(),
        );
    }
}

/// Expand tilde (`~`) and `${ENV}` patterns in an include path.
fn expand_tilde_and_env(path: &str) -> String {
    let mut result = path.to_owned();

    // Expand `~` or `~/`.
    if (result.starts_with("~/") || result == "~")
        && let Some(home) = dirs::home_dir()
    {
        result = result.replacen('~', &home.display().to_string(), 1);
    }

    // Expand `${ENV_VAR}` and `$ENV_VAR`.
    result = expand_env_vars(&result);

    result
}

/// Expand environment variables in `${VAR}` and `$VAR` formats.
///
/// Uses a single-pass builder to avoid repeated string reallocations.
/// If a `${` is encountered without a matching `}`, the literal `${` is
/// preserved in the output to avoid silently corrupting paths.
fn expand_env_vars(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.char_indices().peekable();

    while let Some((i, ch)) = chars.next() {
        if ch == '$' {
            if let Some((_, '{')) = chars.peek() {
                // `${VAR}` form — look for closing `}`.
                chars.next(); // consume '{'
                let start = i + 2;
                if let Some(end_offset) = s[start..].find('}') {
                    let var_name = &s[start..start + end_offset];
                    result.push_str(&std::env::var(var_name).unwrap_or_default());
                    // Skip to after '}'
                    for _ in 0..=end_offset {
                        chars.next();
                    }
                    continue;
                }
                // No closing `}` — preserve the literal `${` to avoid corruption.
                result.push(ch);
                result.push('{');
                continue;
            }
            // `$VAR` form (without braces) — read until non-alphanumeric/underscore.
            let rest = &s[i + 1..];
            let end = rest
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .unwrap_or(rest.len());
            if end > 0 {
                let var_name = &rest[..end];
                result.push_str(&std::env::var(var_name).unwrap_or_default());
                // Skip the variable name characters.
                for _ in 0..end {
                    chars.next();
                }
                continue;
            }
            // Bare `$` at end of string or before non-name char.
            result.push(ch);
        } else {
            result.push(ch);
        }
    }

    result
}

/// Expand glob patterns and return matching file paths.
fn glob_paths(pattern: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Use a simple glob implementation.
    if let Some(parent) = Path::new(pattern).parent() {
        let file_name = Path::new(pattern)
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();

        if let Ok(entries) = std::fs::read_dir(parent) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if simple_glob_match(&name_str, &file_name) {
                    paths.push(entry.path());
                }
            }
        }
    }

    paths.sort();
    paths
}

/// Simple glob match for file names.
///
/// Performs case-sensitive matching, which is correct for Unix file paths.
fn simple_glob_match(name: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') && !pattern.contains('?') {
        return name == pattern;
    }
    // Delegate to the directives glob matcher (case-sensitive for file paths).
    super::directives::glob_matches(name, pattern)
}

/// Apply first-match-wins resolution from a block's nodes.
///
/// Accumulative directives (IdentityFile, CertificateFile, etc.) are
/// collected across all matching blocks. All other directives use
/// first-match-wins semantics.
fn resolve_block(
    nodes: &[ConfigNode],
    _host: &str,
    _ssh_dir: &Path,
    resolved: &mut ResolvedHost,
    seen: &mut HashSet<String>,
) {
    for node in nodes {
        if let ConfigNode::Directive { keyword, value, .. } = node {
            let key_lower = keyword.to_lowercase();

            // Accumulative directives — always collect.
            if super::directives::is_accumulative(&key_lower) {
                if key_lower == "identityfile"
                    && !resolved.identity_files.iter().any(|f| f == value)
                {
                    resolved.identity_files.push(value.clone());
                }
                resolved
                    .directives
                    .push((keyword.clone(), value.clone()));
                continue;
            }

            // Skip if we already have a value (first-match-wins).
            if seen.contains(&key_lower) {
                continue;
            }

            // Match first, then move key_lower into the set to avoid cloning.
            match key_lower.as_str() {
                "hostname" => resolved.host_name = Some(value.clone()),
                "user" => resolved.user = Some(value.clone()),
                "port" => {
                    resolved.port = value.parse::<u16>().ok();
                }
                "proxyjump" => resolved.proxy_jump = Some(value.clone()),
                _ => {}
            }

            resolved
                .directives
                .push((keyword.clone(), value.clone()));
            seen.insert(key_lower);
        }
    }
}

/// Context for SSH token expansion (`%h`, `%d`, `%l`, etc.).
struct TokenContext<'a> {
    host: &'a str,
    home_dir: &'a str,
    local_hostname: &'a str,
    remote_user: &'a str,
    local_user: &'a str,
    port: &'a str,
    /// Canonical hostname (same as host unless CanonicalizeHostname is enabled).
    canonical_host: &'a str,
    /// Identity file being expanded (`%i` → basename).  `None` when
    /// expanding a non-IdentityFile directive.
    identity_file: Option<&'a str>,
}

/// Expand tokens in resolved values.
///
/// Applies tilde, environment-variable, and SSH token expansion to all
/// directive values that may contain them — including the dedicated
/// fields (`identity_files`, `host_name`, `proxy_jump`) and every
/// entry in the raw `directives` vec whose key is listed in
/// [`TOKEN_EXPANDABLE`].
fn expand_resolved(resolved: &mut ResolvedHost, host: &str, _ssh_dir: &Path) {
    let local_user = whoami();
    let local_hostname = hostname();
    let home_dir = dirs::home_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    let port_str = resolved
        .port
        .map_or_else(|| "22".to_owned(), |p| p.to_string());
    let remote_user = resolved.user.as_deref().unwrap_or(&local_user).to_owned();

    let ctx = TokenContext {
        host,
        home_dir: &home_dir,
        local_hostname: &local_hostname,
        remote_user: &remote_user,
        local_user: &local_user,
        port: &port_str,
        // On first pass canonical_host == host; second pass uses the
        // canonical name (already substituted as `host` by the caller).
        canonical_host: host,
        identity_file: None,
    };

    // Expand dedicated fields.
    for id_file in &mut resolved.identity_files {
        *id_file = expand_tilde_and_env(id_file);
        *id_file = expand_tokens(id_file, &ctx);
        *id_file = collapse_double_percent(id_file);
    }

    if let Some(ref mut hn) = resolved.host_name {
        *hn = expand_tilde_and_env(hn);
        *hn = expand_tokens(hn, &ctx);
        *hn = collapse_double_percent(hn);
    }

    if let Some(ref mut pj) = resolved.proxy_jump {
        *pj = expand_tokens(pj, &ctx);
        *pj = collapse_double_percent(pj);
    }

    // Expand all raw directive values that may contain tokens.
    for (key, value) in &mut resolved.directives {
        let key_lower = key.to_lowercase();
        if TOKEN_EXPANDABLE.contains(&key_lower.as_str())
            || key_lower == "identityfile"
            || key_lower == "hostname"
            || key_lower == "proxyjump"
        {
            let expanded = expand_tilde_and_env(value);
            let expanded = expand_tokens(&expanded, &ctx);
            *value = collapse_double_percent(&expanded);
        }
    }
}

/// Check whether `CanonicalizeHostname` is enabled in the resolved config.
///
/// OpenSSH recognises `yes`, `always`, and `no` (the default).  Any other
/// value is treated as `no`.
fn is_canonicalize_enabled(resolved: &ResolvedHost) -> bool {
    resolved
        .directives
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("canonicalizehostname"))
        .is_some_and(|(_, v)| {
            let lv = v.to_lowercase();
            lv == "yes" || lv == "always"
        })
}

/// Expand SSH tokens in a value string.
///
/// Supported tokens (matching OpenSSH ssh_config(5)):
/// - `%%` → literal `%`
/// - `%C` → hash of connection (host+port+user) — placeholder
/// - `%d` → home directory
/// - `%H` → canonical hostname
/// - `%h` / `%n` → remote host (alias)
/// - `%i` → local username (same as `%u`; see note in implementation)
/// - `%L` → local hostname (short)
/// - `%l` → local hostname (FQDN)
/// - `%p` → remote port
/// - `%r` → remote username
/// - `%T` → remote username (same as %r)
/// - `%u` → local username
///
/// Unknown `%X` sequences and trailing `%` are preserved as-is.
fn expand_tokens(s: &str, ctx: &TokenContext<'_>) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            match chars.peek().copied() {
                Some('%') => {
                    // Keep `%%` as-is; collapse_double_percent handles it later.
                    result.push_str("%%");
                    chars.next();
                }
                Some('C') => {
                    // Connection hash — use a simple hash of host:port:user.
                    chars.next();
                    let hash_input = format!("{}:{}:{}", ctx.host, ctx.port, ctx.local_user);
                    let hash = simple_hash(&hash_input);
                    result.push_str(&hash);
                }
                Some('d') => {
                    chars.next();
                    result.push_str(ctx.home_dir);
                }
                Some('H') => {
                    chars.next();
                    result.push_str(ctx.canonical_host);
                }
                Some('h' | 'n') => {
                    chars.next();
                    result.push_str(ctx.host);
                }
                Some('L') => {
                    // Short hostname (first component before '.').
                    chars.next();
                    let short = ctx.local_hostname.split('.').next().unwrap_or(ctx.local_hostname);
                    result.push_str(short);
                }
                Some('l') => {
                    chars.next();
                    result.push_str(ctx.local_hostname);
                }
                Some('p') => {
                    chars.next();
                    result.push_str(ctx.port);
                }
                Some('r' | 'T') => {
                    // %r and %T both expand to the remote username.
                    chars.next();
                    result.push_str(ctx.remote_user);
                }
                Some('i' | 'u') => {
                    // %i and %u both expand to the local username.
                    // Note: per OpenSSH, %i is the "identity file name" in some
                    // contexts, but when used in IdentityFile paths it would create
                    // a circular reference.  The local username fallback matches
                    // OpenSSH behaviour for IdentityFile and most other directives.
                    chars.next();
                    result.push_str(ctx.local_user);
                }
                _ => {
                    // Unknown token or '%' at end of string — keep as-is.
                    result.push(ch);
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Like [`expand_tokens`] but leaves `%i` sequences untouched.
///
/// Used for the first pass of IdentityFile expansion where `%i` must not
/// be expanded yet (it would create a circular reference when the path
/// itself contains `%i`).
fn expand_tokens_skip_i(s: &str, ctx: &TokenContext<'_>) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            match chars.peek().copied() {
                Some('%') => {
                    result.push_str("%%");
                    chars.next();
                }
                Some('i') => {
                    // Preserve %i for the second pass.
                    result.push_str("%i");
                    chars.next();
                }
                Some('C') => {
                    chars.next();
                    let hash_input = format!("{}:{}:{}", ctx.host, ctx.port, ctx.local_user);
                    result.push_str(&simple_hash(&hash_input));
                }
                Some('d') => { chars.next(); result.push_str(ctx.home_dir); }
                Some('H') => { chars.next(); result.push_str(ctx.canonical_host); }
                Some('h' | 'n') => { chars.next(); result.push_str(ctx.host); }
                Some('L') => {
                    chars.next();
                    result.push_str(ctx.local_hostname.split('.').next().unwrap_or(ctx.local_hostname));
                }
                Some('l') => { chars.next(); result.push_str(ctx.local_hostname); }
                Some('p') => { chars.next(); result.push_str(ctx.port); }
                Some('r' | 'T') => { chars.next(); result.push_str(ctx.remote_user); }
                Some('u') => { chars.next(); result.push_str(ctx.local_user); }
                _ => { result.push(ch); }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Simple hash function for `%C` token (OpenSSH uses SHA-1 of host:port:user).
///
/// Uses a FNV-1a style hash for stability across Rust versions.  This is NOT
/// cryptographically secure — it only needs to be deterministic and collision-
/// resistant enough for socket naming.  OpenSSH uses SHA-1 here, but we avoid
/// the dependency.
fn simple_hash(s: &str) -> String {
    let bytes = s.as_bytes();
    // FNV-1a 64-bit parameters
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let prime: u64 = 0x0100_0000_01b3;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(prime);
    }
    format!("{hash:016x}")
}

/// Replace `%%` with a single `%` (OpenSSH escape convention).
fn collapse_double_percent(s: &str) -> String {
    s.replace("%%", "%")
}

/// Get the current username.
fn whoami() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_owned())
}

/// Get the local hostname.
fn hostname() -> String {
    std::env::var("HOSTNAME").unwrap_or_else(|_| {
        gethostname::gethostname()
            .to_string_lossy()
            .into_owned()
    })
}

/// Check if a hostname matches SSH config patterns (reuses directive logic).
fn host_matches(host: &str, patterns: &[String]) -> bool {
    super::directives::host_matches_patterns(host, patterns)
}

/// Check if `Match` criteria are satisfied for the given host context.
///
/// Supported criteria keywords (case-insensitive):
/// - `host <patterns>` — matches against `target_host` (the hostname being
///   resolved, which may be the canonical name on a second pass).
/// - `originalhost <patterns>` — matches against `original_host` (the
///   alias the user typed, before any canonicalization).
/// - `user <names>` — matches against `target_user` (the remote username;
///   comma-separated, case-insensitive comparison).
///
/// Multiple occurrences of the **same** keyword are OR'd (e.g.
/// `host web host db` matches either "web" or "db").  Different keywords
/// are AND'd (e.g. `user alice host web` requires both to match).
///
/// Returns `true` only when every recognized criterion type matches and at
/// least one recognized criterion is present.  Unrecognized keywords
/// (e.g. `exec`, `localuser`, `address`) are silently skipped.
fn match_criteria_host(
    criteria: &str,
    target_host: &str,
    target_user: &str,
    original_host: &str,
) -> bool {
    let mut tokens = criteria.split_whitespace();

    // Track per-keyword-type state: whether the type appeared and whether
    // at least one occurrence matched (OR within a type).
    let mut has_host = false;
    let mut host_matched = false;
    let mut has_originalhost = false;
    let mut originalhost_matched = false;
    let mut has_user = false;
    let mut user_matched = false;

    while let Some(keyword) = tokens.next() {
        if keyword.eq_ignore_ascii_case("host") {
            if let Some(patterns_str) = tokens.next() {
                has_host = true;
                let patterns: Vec<String> =
                    patterns_str.split(',').map(str::to_owned).collect();
                if host_matches(target_host, &patterns) {
                    host_matched = true;
                }
            }
        } else if keyword.eq_ignore_ascii_case("originalhost") {
            if let Some(patterns_str) = tokens.next() {
                has_originalhost = true;
                let patterns: Vec<String> =
                    patterns_str.split(',').map(str::to_owned).collect();
                if host_matches(original_host, &patterns) {
                    originalhost_matched = true;
                }
            }
        } else if keyword.eq_ignore_ascii_case("user") {
            if let Some(names_str) = tokens.next() {
                has_user = true;
                // OpenSSH matches user names case-insensitively.
                let names: Vec<&str> = names_str.split(',').collect();
                if names
                    .iter()
                    .any(|n| n.eq_ignore_ascii_case(target_user))
                {
                    user_matched = true;
                }
            }
        } else {
            // Unknown criterion keyword — consume its value token and skip.
            // Criteria like `exec`, `localuser`, `address` fall here.
            tokens.next();
        }
    }

    // At least one criterion type must be present.
    let any_known = has_host || has_originalhost || has_user;
    // All present types must have matched (AND across types).
    let all_matched = (!has_host || host_matched)
        && (!has_originalhost || originalhost_matched)
        && (!has_user || user_matched);

    any_known && all_matched
}

#[cfg(test)]
#[path = "resolve.test.rs"]
mod tests;
