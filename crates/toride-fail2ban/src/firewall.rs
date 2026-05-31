//! Firewall diagnostics module.
//!
//! Mostly diagnostic in v1 -- does **not** manually insert firewall rules.
//! The [`FirewallChecker`] inspects whether the tools and structures required
//! by the configured Fail2Ban actions are actually present on the host.
//!
//! # Checks performed
//!
//! - `nft` binary availability when nftables actions are configured
//! - `iptables` / `ip6tables` binary availability when iptables actions are
//!   configured
//! - Existence of the expected nft sets (`nft list set inet fail2ban <name>`)
//! - Existence of the expected iptables chains (`iptables -n -L <chain>`)
//! - IPv6 ban support when IPv6 addresses are in use
//!
//! All commands go through the [`Runner`](crate::command::Runner) trait. No
//! direct `std::process::Command` calls are made anywhere in this module.

use crate::command::Runner;
use crate::report::{Finding, Severity};
use crate::Result;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default nftables table used by Fail2Ban's stock nftables actions.
const NFT_TABLE: &str = "fail2ban";

/// Default iptables chain name created by Fail2Ban's stock iptables actions.
const IPTABLES_CHAIN: &str = "f2b-chain";

// ---------------------------------------------------------------------------
// FirewallChecker
// ---------------------------------------------------------------------------

/// Diagnostic checker for firewall backend availability.
///
/// Borrows a [`Runner`] implementation so it can be used with either the
/// production [`DuctRunner`](crate::command::DuctRunner) or the test
/// [`FakeRunner`](crate::command::FakeRunner).
///
/// # Example
///
/// ```ignore
/// use toride_fail2ban::command::DuctRunner;
/// use toride_fail2ban::firewall::FirewallChecker;
///
/// let runner = DuctRunner::new();
/// let checker = FirewallChecker::new(&runner);
///
/// let findings = checker.diagnose(&["nftables".to_string()]);
/// for f in &findings {
///     println!("[{}] {}", f.severity, f.title);
/// }
/// ```
pub struct FirewallChecker<'a> {
    runner: &'a dyn Runner,
}

impl<'a> FirewallChecker<'a> {
    /// Create a new checker that delegates command execution to `runner`.
    pub fn new(runner: &'a dyn Runner) -> Self {
        Self { runner }
    }

    // -----------------------------------------------------------------------
    // Binary availability probes
    // -----------------------------------------------------------------------

    /// Check whether the `nft` binary is available on the system.
    ///
    /// Runs `nft --version` and returns `true` if the command exits
    /// successfully. Returns `Ok(false)` if the command fails (binary not
    /// found, permission denied, etc.).
    pub fn check_nft_available(&self) -> Result<bool> {
        Ok(self.runner.run("nft", &["--version"])?.success)
    }

    /// Check whether the `iptables` binary is available on the system.
    ///
    /// Runs `iptables --version` and returns `true` on success.
    pub fn check_iptables_available(&self) -> Result<bool> {
        Ok(self.runner.run("iptables", &["--version"])?.success)
    }

    /// Check whether the `ip6tables` binary is available on the system.
    ///
    /// Runs `ip6tables --version` and returns `true` on success.
    pub fn check_ip6tables_available(&self) -> Result<bool> {
        Ok(self.runner.run("ip6tables", &["--version"])?.success)
    }

    // -----------------------------------------------------------------------
    // State probes
    // -----------------------------------------------------------------------

    /// Check whether an nft set exists in the Fail2Ban table.
    ///
    /// Runs `nft list set inet fail2ban <set_name>` and returns `true` if
    /// the command succeeds (i.e. the set is present in the kernel).
    pub fn check_nft_set(&self, set_name: &str) -> Result<bool> {
        let args = &["list", "set", "inet", NFT_TABLE, set_name];
        Ok(self.runner.run("nft", args)?.success)
    }

    /// Check whether an iptables chain exists.
    ///
    /// Runs `iptables -n -L <chain>` and returns `true` if the command
    /// succeeds.
    pub fn check_iptables_chain(&self, chain: &str) -> Result<bool> {
        let args = &["-n", "-L", chain];
        Ok(self.runner.run("iptables", args)?.success)
    }

    // -----------------------------------------------------------------------
    // Aggregate diagnosis
    // -----------------------------------------------------------------------

    /// Run all firewall-related diagnostic checks based on the configured
    /// actions and return a list of [`Finding`] values.
    ///
    /// `configured_actions` should contain the action names as they appear in
    /// the Fail2Ban jail configuration (e.g. `"nftables"`, `"iptables"`,
    /// `"iptables-multiport"`, `"nftables-multiport"`).
    ///
    /// # Checks performed
    ///
    /// 1. If any action contains `"nftables"`, verify `nft` is available and
    ///    the expected nft set exists.
    /// 2. If any action contains `"iptables"`, verify `iptables` is available
    ///    and the expected iptables chain exists.
    /// 3. If any action contains `"iptables"` (which implies potential IPv6
    ///    usage), verify `ip6tables` is available as well.
    /// 4. If no firewall-related action is found at all, emit an informational
    ///    note that firewall diagnostics were skipped.
    pub fn diagnose(&self, configured_actions: &[String]) -> Vec<Finding> {
        let mut findings = Vec::new();

        let uses_nft = configured_actions
            .iter()
            .any(|a| a.to_ascii_lowercase().contains("nftables"));

        let uses_iptables = configured_actions
            .iter()
            .any(|a| a.to_ascii_lowercase().contains("iptables"));

        // If no firewall-related actions are configured, emit a single info
        // finding and return early.
        if !uses_nft && !uses_iptables {
            findings.push(
                Finding::new(
                    "firewall.no-firewall-action",
                    Severity::Info,
                    "No firewall action configured",
                )
                .detail(
                    "None of the configured actions reference nftables or \
                     iptables. Firewall backend checks were skipped.",
                ),
            );
            return findings;
        }

        // --- nftables checks ------------------------------------------------

        if uses_nft {
            self.diagnose_nftables(&mut findings);
        }

        // --- iptables checks ------------------------------------------------

        if uses_iptables {
            self.diagnose_iptables(&mut findings);
        }

        findings
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Run nftables-specific diagnostic probes and append findings.
    fn diagnose_nftables(&self, findings: &mut Vec<Finding>) {
        match self.check_nft_available() {
            Ok(true) => {
                // nft is present -- check that the expected set exists.
                match self.check_nft_set(IPTABLES_CHAIN) {
                    Ok(true) => {
                        findings.push(Finding::new(
                            "firewall.nft.set-present",
                            Severity::Ok,
                            "nft fail2ban set exists",
                        ));
                    }
                    Ok(false) => {
                        findings.push(
                            Finding::new(
                                "firewall.nft.set-missing",
                                Severity::Warning,
                                "nft fail2ban set not found",
                            )
                            .detail(format!(
                                "The nft set \"{}\" in table \"inet {}\" does \
                                 not exist. It is normally created when a jail \
                                 with a nftables action starts.",
                                IPTABLES_CHAIN, NFT_TABLE,
                            ))
                            .fix(
                                "Start (or restart) the jail so Fail2Ban \
                                 creates the nft set, or verify that the \
                                 correct action is configured.",
                            ),
                        );
                    }
                    Err(e) => {
                        findings.push(
                            Finding::new(
                                "firewall.nft.set-check-error",
                                Severity::Error,
                                "Failed to check nft set",
                            )
                            .detail(format!(
                                "Running `nft list set inet {} {}` failed: {e}",
                                NFT_TABLE, IPTABLES_CHAIN,
                            )),
                        );
                    }
                }
            }
            Ok(false) => {
                findings.push(
                    Finding::new(
                        "firewall.nft.missing",
                        Severity::Critical,
                        "nft binary not available",
                    )
                    .detail(
                        "A nftables action is configured but the `nft` binary \
                         could not be executed. Bans will not be enforced at \
                         the firewall level.",
                    )
                    .fix("Install nftables: apt install nftables (Debian/Ubuntu) or dnf install nftables (Fedora)."),
                );
            }
            Err(e) => {
                findings.push(
                    Finding::new(
                        "firewall.nft.check-error",
                        Severity::Error,
                        "Failed to check nft availability",
                    )
                    .detail(format!("Running `nft --version` failed: {e}")),
                );
            }
        }
    }

    /// Run iptables-specific diagnostic probes and append findings.
    fn diagnose_iptables(&self, findings: &mut Vec<Finding>) {
        // --- iptables (IPv4) ------------------------------------------------

        match self.check_iptables_available() {
            Ok(true) => {
                // iptables is present -- check the expected chain.
                match self.check_iptables_chain(IPTABLES_CHAIN) {
                    Ok(true) => {
                        findings.push(Finding::new(
                            "firewall.iptables.chain-present",
                            Severity::Ok,
                            "iptables fail2ban chain exists",
                        ));
                    }
                    Ok(false) => {
                        findings.push(
                            Finding::new(
                                "firewall.iptables.chain-missing",
                                Severity::Warning,
                                "iptables fail2ban chain not found",
                            )
                            .detail(format!(
                                "The iptables chain \"{IPTABLES_CHAIN}\" does not \
                                 exist. It is normally created when a jail with \
                                 an iptables action starts.",
                            ))
                            .fix(
                                "Start (or restart) the jail so Fail2Ban \
                                 creates the chain, or verify that the correct \
                                 action is configured.",
                            ),
                        );
                    }
                    Err(e) => {
                        findings.push(
                            Finding::new(
                                "firewall.iptables.chain-check-error",
                                Severity::Error,
                                "Failed to check iptables chain",
                            )
                            .detail(format!(
                                "Running `iptables -n -L {IPTABLES_CHAIN}` \
                                 failed: {e}",
                            )),
                        );
                    }
                }
            }
            Ok(false) => {
                findings.push(
                    Finding::new(
                        "firewall.iptables.missing",
                        Severity::Critical,
                        "iptables binary not available",
                    )
                    .detail(
                        "An iptables action is configured but the `iptables` \
                         binary could not be executed. Bans will not be \
                         enforced at the firewall level.",
                    )
                    .fix("Install iptables: apt install iptables (Debian/Ubuntu) or dnf install iptables (Fedora)."),
                );
            }
            Err(e) => {
                findings.push(
                    Finding::new(
                        "firewall.iptables.check-error",
                        Severity::Error,
                        "Failed to check iptables availability",
                    )
                    .detail(format!("Running `iptables --version` failed: {e}")),
                );
            }
        }

        // --- ip6tables (IPv6) -----------------------------------------------

        match self.check_ip6tables_available() {
            Ok(true) => {
                findings.push(Finding::new(
                    "firewall.ip6tables.available",
                    Severity::Ok,
                    "ip6tables binary available (IPv6 ban support)",
                ));
            }
            Ok(false) => {
                findings.push(
                    Finding::new(
                        "firewall.ip6tables.missing",
                        Severity::Warning,
                        "ip6tables binary not available",
                    )
                    .detail(
                        "An iptables action is configured but `ip6tables` is \
                         not available. IPv6 addresses will not be banned at \
                         the firewall level.",
                    )
                    .fix("Install ip6tables: apt install iptables (Debian/Ubuntu) or dnf install iptables (Fedora)."),
                );
            }
            Err(e) => {
                findings.push(
                    Finding::new(
                        "firewall.ip6tables.check-error",
                        Severity::Error,
                        "Failed to check ip6tables availability",
                    )
                    .detail(format!("Running `ip6tables --version` failed: {e}")),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "firewall.test.rs"]
mod tests;
