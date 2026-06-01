//! Severity levels for diagnostic findings.

use std::fmt;
use std::str::FromStr;

/// How serious a diagnostic finding is.
///
/// Ordinal ordering: `Critical` > `Important` > `Warning` > `Info` > `Ok`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Severity {
    /// The check passed successfully.
    Ok,
    /// Informational note, no action required.
    Info,
    /// A non-critical issue or potential problem.
    Warning,
    /// A significant issue that should be addressed soon.
    Important,
    /// The system is in a broken or insecure state that must be fixed immediately.
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (emoji, label) = match self {
            Self::Critical => ("\u{1F534}", "CRITICAL"),
            Self::Important => ("\u{1F7E1}", "IMPORTANT"),
            Self::Warning => ("\u{1F7E3}", "WARNING"),
            Self::Info => ("\u{1F535}", "INFO"),
            Self::Ok => ("\u{2705}", "OK"),
        };
        write!(f, "{emoji} {label}")
    }
}

impl FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "critical" => Ok(Self::Critical),
            "important" => Ok(Self::Important),
            "warning" => Ok(Self::Warning),
            "info" => Ok(Self::Info),
            "ok" => Ok(Self::Ok),
            other => Err(format!("unknown severity: {other}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering() {
        assert!(Severity::Critical > Severity::Important);
        assert!(Severity::Important > Severity::Warning);
        assert!(Severity::Warning > Severity::Info);
        assert!(Severity::Info > Severity::Ok);
    }

    #[test]
    fn display_contains_emoji() {
        assert!(Severity::Critical.to_string().contains('\u{1F534}'));
        assert!(Severity::Ok.to_string().contains('\u{2705}'));
    }

    #[test]
    fn from_str_roundtrip() {
        for (label, expected) in [
            ("critical", Severity::Critical),
            ("important", Severity::Important),
            ("warning", Severity::Warning),
            ("info", Severity::Info),
            ("ok", Severity::Ok),
        ] {
            assert_eq!(Severity::from_str(label).unwrap(), expected);
        }
    }

    #[test]
    fn from_str_case_insensitive() {
        assert_eq!(Severity::from_str("CRITICAL").unwrap(), Severity::Critical);
        assert_eq!(Severity::from_str("Warning").unwrap(), Severity::Warning);
    }

    #[test]
    fn from_str_unknown() {
        assert!(Severity::from_str("bogus").is_err());
    }
}
