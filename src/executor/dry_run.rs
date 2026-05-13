use crate::tui::model::Plan;

pub fn dry_run_report(plan: &Plan) -> String {
    let mut report = String::from("Dry Run Report\n══════════════\n\n");
    report.push_str("No changes have been applied.\n\n");
    report.push_str("Planned actions:\n");

    for (i, action) in plan.actions.iter().enumerate() {
        report.push_str(&format!("{}. [{}] {}\n", i + 1, action.module_id.label(), action.label));
    }

    report
}
