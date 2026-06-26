//! Core synchronous [`Runner`] trait.
//!
//! Every command executor in the workspace implements this trait,
//! allowing callers to swap between real and fake implementations
//! without changing downstream logic.

use crate::error::Result;
use crate::output::CommandOutput;
use crate::spec::CommandSpec;

/// A trait for executing commands synchronously.
///
/// Implementors provide the actual execution strategy (e.g. spawning a
/// subprocess via `duct`, returning canned responses for tests, etc.).
pub trait Runner: Send + Sync {
    /// Execute the given [`CommandSpec`] and return its output.
    fn run(&self, spec: &CommandSpec) -> Result<CommandOutput>;

    /// Execute the command and return an error if it did not succeed.
    ///
    /// The default implementation calls [`Runner::run`] and checks the
    /// `success` flag on the returned [`CommandOutput`].
    fn run_checked(&self, spec: &CommandSpec) -> Result<CommandOutput> {
        let output = self.run(spec)?;
        if !output.success {
            let args = if spec.redact {
                crate::display::display_command(spec, &[])
            } else {
                spec.args.join(" ")
            };
            return Err(crate::error::Error::CommandFailed {
                program: spec.program.clone(),
                args,
                exit_code: output.exit_code,
                stderr: output.stderr.clone(),
            });
        }
        Ok(output)
    }
}
