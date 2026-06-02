//! Tool alias management via `mise tool-alias`.
//!
//! Exposes [`ToolAlias`] for deserialised alias entries and adds
//! [`Mise::tool_aliases_list`], [`Mise::tool_alias_get`],
//! [`Mise::tool_alias_set`], and [`Mise::tool_alias_unset`] methods
//! on the client.

use serde::Deserialize;

use crate::client::Mise;
use crate::error::MiseResult;

// ---------------------------------------------------------------------------
// JSON response types
// ---------------------------------------------------------------------------

/// A single tool alias entry as returned by `mise tool-alias ls --output=json`.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolAlias {
    /// The tool name the alias belongs to (e.g. `"node"`).
    pub tool: String,
    /// The alias name (e.g. `"lts"`).
    pub alias: String,
    /// The version the alias resolves to (e.g. `"22"`).
    pub version: String,
}

// ---------------------------------------------------------------------------
// Mise methods
// ---------------------------------------------------------------------------

impl Mise {
    /// List tool aliases, optionally filtered by tool name.
    ///
    /// When `tool` is `Some`, only aliases for that tool are returned by passing
    /// `--tool <name>` to the CLI.
    ///
    /// Invokes `mise tool-alias ls [--tool <tool>] --output=json`.
    ///
    /// # Errors
    ///
    /// Returns [`MiseError::CommandFailed`] if the command exits non-zero.
    /// Returns [`MiseError::JsonParse`] if the output cannot be deserialised.
    pub async fn tool_aliases_list(&self, tool: Option<&str>) -> MiseResult<Vec<ToolAlias>> {
        let mut args: Vec<String> = vec!["tool-alias".into(), "ls".into()];
        if let Some(t) = tool {
            args.push("--tool".into());
            args.push(t.into());
        }
        args.push("--output=json".into());
        self.run_json(args).await
    }

    /// Get the version a tool alias resolves to.
    ///
    /// Invokes `mise tool-alias get <tool> <alias>`.
    ///
    /// # Errors
    ///
    /// Returns [`MiseError::CommandFailed`] if the command exits non-zero.
    pub async fn tool_alias_get(&self, tool: &str, alias: &str) -> MiseResult<String> {
        let output = self
            .run_checked(["tool-alias", "get", tool, alias])
            .await?;
        Ok(output.stdout_trimmed().to_owned())
    }

    /// Set (create or update) a tool alias.
    ///
    /// Invokes `mise tool-alias set <tool> <alias> <version>`.
    ///
    /// # Errors
    ///
    /// Returns [`MiseError::CommandFailed`] if the command exits non-zero.
    pub async fn tool_alias_set(
        &self,
        tool: &str,
        alias: &str,
        version: &str,
    ) -> MiseResult<()> {
        self.run_checked(["tool-alias", "set", tool, alias, version])
            .await?;
        Ok(())
    }

    /// Remove a tool alias.
    ///
    /// Invokes `mise tool-alias unset <tool> <alias>`.
    ///
    /// # Errors
    ///
    /// Returns [`MiseError::CommandFailed`] if the command exits non-zero.
    pub async fn tool_alias_unset(&self, tool: &str, alias: &str) -> MiseResult<()> {
        self.run_checked(["tool-alias", "unset", tool, alias])
            .await?;
        Ok(())
    }
}
