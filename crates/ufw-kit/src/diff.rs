//! Diff utilities for comparing file contents before and after changes.

/// Compute a unified diff between old and new content.
#[must_use]
pub fn unified_diff(old: &str, new: &str, context: &str) -> String {
    similar::TextDiff::from_lines(old, new)
        .unified_diff()
        .context_radius(3)
        .header(&format!("a/{context}"), &format!("b/{context}"))
        .to_string()
}

/// Check if two strings are identical.
#[must_use]
pub fn is_identical(old: &str, new: &str) -> bool {
    old == new
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unified_diff_should_show_changes() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nmodified\nline3\n";
        let diff = unified_diff(old, new, "test.txt");
        assert!(diff.contains("-line2"));
        assert!(diff.contains("+modified"));
    }

    #[test]
    fn is_identical_should_return_true_for_same_content() {
        assert!(is_identical("hello", "hello"));
    }

    #[test]
    fn is_identical_should_return_false_for_different_content() {
        assert!(!is_identical("hello", "world"));
    }
}
