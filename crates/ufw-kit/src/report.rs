//! Report rendering — converts doctor findings into human-readable output.

use crate::spec::{Finding, Severity};

/// Render a list of findings as a human-readable text report.
#[must_use]
pub fn render_findings(findings: &[Finding]) -> String {
    let mut out = String::new();

    out.push_str("=== UFW Doctor Report ===\n\n");

    let critical = findings.iter().filter(|f| f.severity == Severity::Critical).count();
    let errors = findings.iter().filter(|f| f.severity == Severity::Error).count();
    let warnings = findings.iter().filter(|f| f.severity == Severity::Warning).count();
    let ok = findings.iter().filter(|f| f.severity == Severity::Ok).count();
    let info = findings.iter().filter(|f| f.severity == Severity::Info).count();

    out.push_str(&format!(
        "Summary: {ok} OK, {info} info, {warnings} warnings, {errors} errors, {critical} critical\n\n"
    ));

    // Show critical and errors first
    for finding in findings.iter().filter(|f| f.severity >= Severity::Error) {
        out.push_str(&format!("[{}] {}\n", finding.severity, finding.title));
        out.push_str(&format!("  {}\n", finding.detail));
        if let Some(fix) = &finding.fix {
            out.push_str(&format!("  Fix: {fix}\n"));
        }
        out.push('\n');
    }

    // Then warnings
    for finding in findings.iter().filter(|f| f.severity == Severity::Warning) {
        out.push_str(&format!("[{}] {}\n", finding.severity, finding.title));
        out.push_str(&format!("  {}\n", finding.detail));
        if let Some(fix) = &finding.fix {
            out.push_str(&format!("  Fix: {fix}\n"));
        }
        out.push('\n');
    }

    // Then info and OK
    for finding in findings.iter().filter(|f| f.severity <= Severity::Info) {
        out.push_str(&format!("[{}] {}\n", finding.severity, finding.title));
        if !finding.detail.is_empty() {
            out.push_str(&format!("  {}\n", finding.detail));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_findings_should_show_summary() {
        let findings = vec![
            Finding {
                id: "test:ok",
                severity: Severity::Ok,
                title: "All good".into(),
                detail: "Everything is fine.".into(),
                fix: None,
            },
            Finding {
                id: "test:warn",
                severity: Severity::Warning,
                title: "Watch out".into(),
                detail: "Something might be wrong.".into(),
                fix: Some("Fix it.".into()),
            },
        ];

        let report = render_findings(&findings);
        assert!(report.contains("1 OK"));
        assert!(report.contains("1 warnings"));
        assert!(report.contains("All good"));
        assert!(report.contains("Watch out"));
        assert!(report.contains("Fix it."));
    }
}
