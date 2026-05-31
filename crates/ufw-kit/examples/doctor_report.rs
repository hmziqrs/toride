//! Example: Run a full doctor report and print findings.

use ufw_kit::{Ufw, spec::DoctorScope};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ufw = Ufw::system();

    println!("Running UFW doctor diagnostics...\n");
    let findings = ufw_kit::doctor::doctor(&ufw, DoctorScope::All)?;

    if findings.is_empty() {
        println!("No issues found - UFW looks healthy!");
    } else {
        for finding in &findings {
            println!("[{}] {}", finding.severity, finding.title);
            println!("  {}", finding.detail);
            if let Some(fix) = &finding.fix {
                println!("  Fix: {fix}");
            }
            println!();
        }
    }

    Ok(())
}
