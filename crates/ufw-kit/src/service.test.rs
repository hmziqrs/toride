use super::*;
use crate::command::FakeRunner;

// ---------------------------------------------------------------------------
// is_active
// ---------------------------------------------------------------------------

#[test]
fn is_active_should_return_true_when_active() {
    let runner = FakeRunner::new().respond_ok("systemctl", &["is-active", "ufw"], "active\n");
    assert!(is_active(&runner).unwrap());
}

#[test]
fn is_active_should_return_false_when_inactive() {
    let runner = FakeRunner::new().respond_ok("systemctl", &["is-active", "ufw"], "inactive\n");
    assert!(!is_active(&runner).unwrap());
}

// ---------------------------------------------------------------------------
// is_enabled
// ---------------------------------------------------------------------------

#[test]
fn is_enabled_should_return_true_when_enabled() {
    let runner = FakeRunner::new().respond_ok("systemctl", &["is-enabled", "ufw"], "enabled\n");
    assert!(is_enabled(&runner).unwrap());
}

#[test]
fn is_enabled_should_return_false_when_disabled() {
    let runner = FakeRunner::new().respond_ok("systemctl", &["is-enabled", "ufw"], "disabled\n");
    assert!(!is_enabled(&runner).unwrap());
}

// ---------------------------------------------------------------------------
// Edge-case tests
// ---------------------------------------------------------------------------

#[test]
fn is_active_should_handle_extra_whitespace() {
    let runner = FakeRunner::new().respond_ok("systemctl", &["is-active", "ufw"], "  active  \n");
    assert!(is_active(&runner).unwrap());
}

// ---------------------------------------------------------------------------
// Weird edge-case tests
// ---------------------------------------------------------------------------

#[test]
fn is_active_should_handle_failed_command() {
    let runner = FakeRunner::new().respond_err("systemctl", &["is-active", "ufw"], "not found", 4);
    // The command runs but returns non-"active"
    assert!(!is_active(&runner).unwrap());
}
