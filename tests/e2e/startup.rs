use std::time::Duration;

use testty::prelude::*;

fn spawn_toride(cols: u16, rows: u16) -> PtySession {
    PtySessionBuilder::new(env!("CARGO_BIN_EXE_toride"))
        .size(cols, rows)
        .env("TORIDE_E2E", "1")
        .env("TORIDE_NO_ANIM", "1")
        .env("TERM", "xterm-256color")
        .spawn()
        .expect("failed to spawn toride")
}

#[test]
fn startup_shows_welcome_and_quit() {
    let mut session = spawn_toride(100, 32);

    session
        .wait_for_text("Toride", Duration::from_secs(10))
        .expect("should show Toride title");

    session
        .wait_for_text("VPS Setup Tool", Duration::from_secs(3))
        .expect("should show subtitle");

    session.press_key("q").expect("press q to quit");

    let exited = session
        .wait_for_exit(Duration::from_secs(3))
        .expect("wait for exit");
    assert!(exited, "process should have exited");
}

#[test]
fn startup_transitions_to_profile_select() {
    let mut session = spawn_toride(100, 32);

    session
        .wait_for_text("Choose setup profile", Duration::from_secs(10))
        .expect("should reach profile select screen");

    session
        .wait_for_text("Basic", Duration::from_secs(3))
        .expect("should show Basic option");

    session
        .wait_for_text("Custom", Duration::from_secs(3))
        .expect("should show Custom option");

    session.press_key("q").expect("press q");
    let _ = session.wait_for_exit(Duration::from_secs(3));
}

#[test]
fn ctrl_c_exits() {
    let mut session = spawn_toride(100, 32);

    session
        .wait_for_text("Toride", Duration::from_secs(10))
        .expect("should start");

    session.press_key("ctrl+c").expect("press ctrl+c");

    let exited = session
        .wait_for_exit(Duration::from_secs(3))
        .expect("wait for exit");
    assert!(exited, "ctrl+c should exit the app");
}
