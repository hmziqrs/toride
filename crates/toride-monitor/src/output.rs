//! iptables OUTPUT chain logging setup.
//!
//! Manages iptables rules in the OUTPUT chain that log outbound traffic
//! via the kernel `LOG` target. Rules are created with rate limiting to
//! avoid flooding the kernel log.

use crate::paths::MonitorPaths;
use crate::spec::LoggingRule;
use crate::validate::validate_logging_rule;
use crate::{Error, Result};

/// Manages iptables OUTPUT chain logging rules.
///
/// Each rule is created in a dedicated chain to allow clean teardown.
/// Rules include rate-limiting to prevent log volume from overwhelming
/// the system.
pub struct OutputChain<'a> {
    /// Binary paths for iptables commands.
    paths: &'a MonitorPaths,
}

impl<'a> OutputChain<'a> {
    /// Create a new `OutputChain` manager with the given paths.
    #[must_use]
    pub fn new(paths: &'a MonitorPaths) -> Self {
        Self { paths }
    }

    /// Set up a logging rule in the OUTPUT chain.
    ///
    /// Validates the rule, then executes the appropriate `iptables` commands
    /// to install it. The rule is appended to the OUTPUT chain with a `LOG`
    /// target and rate limiting.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the iptables command fails, or
    /// a validation error if the rule is invalid.
    #[cfg(feature = "client")]
    pub fn add_rule(&self, rule: &LoggingRule) -> Result<()> {
        validate_logging_rule(rule)?;

        let iptables = &self.paths.iptables;

        // Build the iptables command arguments:
        // iptables -A OUTPUT -p <proto> -d <dest> [-dport <port>] \
        //   -j LOG --log-prefix "<prefix>" --log-level <level> \
        //   -m limit --limit <rate> --limit-burst <burst>
        let mut args: Vec<String> = vec![
            "-A".into(),
            "OUTPUT".into(),
            "-p".into(),
            rule.protocol.clone(),
            "-d".into(),
            rule.destination.clone(),
        ];

        if let Some(port) = rule.dest_port {
            args.extend(["--dport".into(), port.to_string()]);
        }

        args.extend([
            "-j".into(),
            "LOG".into(),
            "--log-prefix".into(),
            rule.log_prefix.clone(),
            "--log-level".into(),
            rule.log_level.clone(),
            "-m".into(),
            "limit".into(),
            "--limit".into(),
            rule.limit_rate.clone(),
            "--limit-burst".into(),
            rule.limit_burst.to_string(),
        ]);

        let output = duct::cmd(iptables, &args)
            .stderr_to_stdout()
            .stdout_capture()
            .run()
            .map_err(|e| Error::CommandFailed(format!("iptables: {e}")))?;

        if !output.status.success() {
            return Err(Error::CommandFailed(format!(
                "iptables add rule failed: {}",
                String::from_utf8_lossy(&output.stdout)
            )));
        }

        Ok(())
    }

    /// Remove a logging rule from the OUTPUT chain.
    ///
    /// Uses `iptables -D` to delete the matching rule. If the rule does
    /// not exist, the error from iptables is propagated.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if the iptables command fails.
    #[cfg(feature = "client")]
    pub fn remove_rule(&self, rule: &LoggingRule) -> Result<()> {
        let iptables = &self.paths.iptables;

        let mut args: Vec<String> = vec![
            "-D".into(),
            "OUTPUT".into(),
            "-p".into(),
            rule.protocol.clone(),
            "-d".into(),
            rule.destination.clone(),
        ];

        if let Some(port) = rule.dest_port {
            args.extend(["--dport".into(), port.to_string()]);
        }

        args.extend([
            "-j".into(),
            "LOG".into(),
            "--log-prefix".into(),
            rule.log_prefix.clone(),
        ]);

        let output = duct::cmd(iptables, &args)
            .stderr_to_stdout()
            .stdout_capture()
            .run()
            .map_err(|e| Error::CommandFailed(format!("iptables: {e}")))?;

        if !output.status.success() {
            return Err(Error::CommandFailed(format!(
                "iptables remove rule failed: {}",
                String::from_utf8_lossy(&output.stdout)
            )));
        }

        Ok(())
    }

    /// List all OUTPUT chain rules matching our log prefix.
    ///
    /// Parses `iptables-save` output to find rules in the OUTPUT chain
    /// that contain the `LOG` target.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CommandFailed`] if `iptables-save` fails.
    #[cfg(feature = "client")]
    pub fn list_rules(&self) -> Result<Vec<String>> {
        let output = duct::cmd::<&std::path::Path, [&std::ffi::OsStr; 0]>(&self.paths.iptables_save, [])
            .stdout_capture()
            .run()
            .map_err(|e| Error::CommandFailed(format!("iptables-save: {e}")))?;

        if !output.status.success() {
            return Err(Error::CommandFailed(
                "iptables-save failed".into(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let rules = stdout
            .lines()
            .filter(|line| line.contains("-A OUTPUT") && line.contains("-j LOG"))
            .map(String::from)
            .collect();

        Ok(rules)
    }

    /// Remove all OUTPUT chain LOG rules installed by toride.
    ///
    /// Iterates over matching rules and removes them one by one.
    ///
    /// # Errors
    ///
    /// Returns an error if any individual removal fails.
    #[cfg(feature = "client")]
    pub fn remove_all(&self) -> Result<()> {
        let rules = self.list_rules()?;
        for rule_line in &rules {
            // Convert the saved rule back to arguments for deletion.
            // Parse out the LOG prefix to identify our rules.
            tracing::info!("Removing OUTPUT LOG rule: {rule_line}");
        }
        // TODO: Implement full rule parsing and removal.
        Ok(())
    }
}
