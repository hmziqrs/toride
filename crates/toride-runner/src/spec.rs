//! [`CommandSpec`] — a declarative description of a command to run.
//!
//! Use the builder-style methods to construct a spec, then pass it to
//! any [`Runner`](crate::Runner) implementation.

use std::time::Duration;

/// A declarative specification of a command to execute.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use toride_runner::CommandSpec;
///
/// let spec = CommandSpec::new("ufw")
///     .arg("status")
///     .timeout(Duration::from_secs(10));
/// ```
#[derive(Debug, Clone)]
pub struct CommandSpec {
    /// The program to execute (looked up via `$PATH` unless absolute).
    pub program: String,
    /// Positional arguments to pass to the program.
    pub args: Vec<String>,
    /// Optional data to pipe to the process's stdin.
    pub stdin: Option<String>,
    /// Optional wall-clock timeout for the command.
    pub timeout: Option<Duration>,
    /// Extra environment variables (`(key, value)` pairs).
    pub env: Vec<(String, String)>,
}

impl CommandSpec {
    /// Create a new spec for the given program with no arguments.
    #[must_use]
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            stdin: None,
            timeout: None,
            env: Vec::new(),
        }
    }

    /// Append a single argument.
    #[must_use]
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Append multiple arguments.
    #[must_use]
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Set stdin data for the process.
    #[must_use]
    pub fn stdin(mut self, data: impl Into<String>) -> Self {
        self.stdin = Some(data.into());
        self
    }

    /// Set a wall-clock timeout.
    #[must_use]
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Add an environment variable.
    #[must_use]
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Add multiple environment variables.
    #[must_use]
    pub fn envs<I, K, V>(mut self, pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.env
            .extend(pairs.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for CommandSpec {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("CommandSpec", 5)?;
        s.serialize_field("program", &self.program)?;
        s.serialize_field("args", &self.args)?;
        s.serialize_field("stdin", &self.stdin)?;
        s.serialize_field("timeout", &self.timeout.map(|d| d.as_secs()))?;
        s.serialize_field("env", &self.env)?;
        s.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for CommandSpec {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct CommandSpecHelper {
            program: String,
            args: Vec<String>,
            stdin: Option<String>,
            timeout: Option<u64>,
            env: Vec<(String, String)>,
        }

        let h = CommandSpecHelper::deserialize(deserializer)?;
        Ok(CommandSpec {
            program: h.program,
            args: h.args,
            stdin: h.stdin,
            timeout: h.timeout.map(Duration::from_secs),
            env: h.env,
        })
    }
}
