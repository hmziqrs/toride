//! Audit rule management.
//!
//! Provides functions for reading, writing, and managing audit rule files
//! in `/etc/audit/rules.d/`.

use std::fs;

use crate::{AuditPaths, Error, Result};

// ---------------------------------------------------------------------------
// AuditRuleFile
// ---------------------------------------------------------------------------

/// Represents an audit rules file in `/etc/audit/rules.d/`.
#[derive(Debug, Clone)]
pub struct AuditRuleFile {
    /// File name (without directory).
    pub name: String,
    /// Raw content of the rules file.
    pub content: String,
}

impl AuditRuleFile {
    /// Parse the content into individual rule lines.
    ///
    /// Filters out empty lines and comments.
    #[must_use]
    pub fn rules(&self) -> Vec<&str> {
        self.content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with('#')
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Rule management functions
// ---------------------------------------------------------------------------

/// List all audit rule files in the rules directory.
///
/// Returns files with the `.rules` extension, sorted alphabetically.
///
/// # Errors
///
/// Returns [`Error::Io`] if the directory cannot be read.
pub fn list_rule_files(paths: &AuditPaths) -> Result<Vec<AuditRuleFile>> {
    if !paths.rules_d.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(&paths.rules_d)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|ext| ext == "rules") {
            let name = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let content = fs::read_to_string(&path)?;
            files.push(AuditRuleFile { name, content });
        }
    }

    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

/// Read a specific audit rule file by name.
///
/// # Errors
///
/// Returns [`Error::Io`] if the file cannot be read.
pub fn read_rule_file(paths: &AuditPaths, name: &str) -> Result<AuditRuleFile> {
    let path = paths.rules_path(name);
    let content = fs::read_to_string(&path)?;
    Ok(AuditRuleFile {
        name: name.to_owned(),
        content,
    })
}

/// Write an audit rule file, creating a backup first.
///
/// # Errors
///
/// Returns [`Error::ConfigWrite`] if the file cannot be written.
pub fn write_rule_file(paths: &AuditPaths, name: &str, content: &str) -> Result<()> {
    let path = paths.rules_path(name);

    if path.exists() {
        crate::backup::create_backup(&path)?;
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&path, content).map_err(|e| Error::ConfigWrite(format!("{e}")))?;
    Ok(())
}

/// Remove an audit rule file.
///
/// Creates a backup before removing.
///
/// # Errors
///
/// Returns [`Error::Io`] if the file cannot be removed.
pub fn remove_rule_file(paths: &AuditPaths, name: &str) -> Result<()> {
    let path = paths.rules_path(name);

    if path.exists() {
        crate::backup::create_backup(&path)?;
        fs::remove_file(&path)?;
    }

    Ok(())
}

/// Merge multiple rule files into a single sorted list of rules.
///
/// Deduplicates rules and sorts them for deterministic output.
#[must_use]
pub fn merge_rules(files: &[AuditRuleFile]) -> Vec<String> {
    let mut rules: Vec<String> = files
        .iter()
        .flat_map(|f| {
            f.content
                .lines()
                .filter(|line| {
                    let trimmed = line.trim();
                    !trimmed.is_empty() && !trimmed.starts_with('#')
                })
                .map(|r| r.to_string())
                .collect::<Vec<_>>()
        })
        .collect();

    rules.sort();
    rules.dedup();
    rules
}
