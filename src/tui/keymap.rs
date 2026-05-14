use crate::tui::model::Screen;

#[derive(Debug, Clone)]
pub struct Binding {
    pub key: String,
    pub action: String,
    pub description: String,
    pub screen: Option<Screen>,
}

pub fn global_bindings() -> Vec<Binding> {
    vec![
        Binding { key: "q".into(), action: "Quit".into(), description: "Quit application".into(), screen: None },
        Binding { key: "Ctrl+C".into(), action: "Cancel".into(), description: "Cancel current operation".into(), screen: None },
        Binding { key: "?".into(), action: "Help".into(), description: "Toggle help overlay".into(), screen: None },
        Binding { key: ":".into(), action: "Palette".into(), description: "Open command palette".into(), screen: None },
        Binding { key: "/".into(), action: "Search".into(), description: "Open search".into(), screen: None },
        Binding { key: "Esc".into(), action: "Back".into(), description: "Go back / close overlay".into(), screen: None },
        Binding { key: "Tab".into(), action: "NextPane".into(), description: "Next pane".into(), screen: None },
        Binding { key: "Shift+Tab".into(), action: "PrevPane".into(), description: "Previous pane".into(), screen: None },
        Binding { key: "Ctrl+S".into(), action: "Save".into(), description: "Save config".into(), screen: None },
        Binding { key: "Ctrl+L".into(), action: "Log".into(), description: "Toggle log panel".into(), screen: None },
        Binding { key: "Ctrl+T".into(), action: "DismissToast".into(), description: "Dismiss top toast".into(), screen: None },
        Binding { key: "Ctrl+R".into(), action: "Reload".into(), description: "Reload config from disk".into(), screen: None },
        Binding { key: "F2".into(), action: "Theme".into(), description: "Cycle theme".into(), screen: None },
    ]
}

pub fn module_selection_bindings() -> Vec<Binding> {
    vec![
        Binding { key: "Space".into(), action: "Toggle".into(), description: "Toggle module".into(), screen: None },
        Binding { key: "Enter".into(), action: "Expand".into(), description: "Expand module details".into(), screen: None },
        Binding { key: "a".into(), action: "SelectAll".into(), description: "Select all".into(), screen: None },
        Binding { key: "n".into(), action: "SelectNone".into(), description: "Select none".into(), screen: None },
        Binding { key: "i".into(), action: "Invert".into(), description: "Invert selection".into(), screen: None },
        Binding { key: "r".into(), action: "Reset".into(), description: "Reset to profile defaults".into(), screen: None },
        Binding { key: "p".into(), action: "Plan".into(), description: "Preview plan".into(), screen: None },
        Binding { key: "d".into(), action: "DryRun".into(), description: "Toggle dry-run mode".into(), screen: None },
        Binding { key: "x".into(), action: "Execute".into(), description: "Proceed to preflight".into(), screen: None },
    ]
}

pub fn apply_bindings() -> Vec<Binding> {
    vec![
        Binding { key: "f".into(), action: "Follow".into(), description: "Toggle follow-tail".into(), screen: None },
        Binding { key: "s".into(), action: "Skip".into(), description: "Skip failed step".into(), screen: None },
        Binding { key: "R".into(), action: "Retry".into(), description: "Retry current step".into(), screen: None },
    ]
}

pub fn help_bindings() -> Vec<Binding> {
    vec![
        Binding { key: "?".into(), action: "Close".into(), description: "Close help".into(), screen: None },
        Binding { key: "Esc".into(), action: "Close".into(), description: "Close help".into(), screen: None },
        Binding { key: "/".into(), action: "Search".into(), description: "Search bindings".into(), screen: None },
    ]
}

pub fn status_bar_hints(screen: Screen, width: u16) -> Vec<Binding> {
    let max_hints = if width >= 100 { 4 } else { 3 };
    let mut hints = global_bindings();

    match screen {
        Screen::ModuleSelect => hints.extend(module_selection_bindings()),
        Screen::Apply => hints.extend(apply_bindings()),
        _ => {}
    }

    hints.truncate(max_hints);
    hints
}
