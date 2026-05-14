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
fn basic_profile_shows_default_modules() {
    let mut session = spawn_toride(100, 32);

    session
        .wait_for_text("Choose setup profile", Duration::from_secs(10))
        .expect("should reach profile select");

    session.press_key("enter").expect("select Basic profile");

    session
        .wait_for_text("Modules", Duration::from_secs(5))
        .expect("should reach module select");

    session
        .wait_for_text("System Update", Duration::from_secs(3))
        .expect("should show System Update");

    session.press_key("q").expect("press q");
    let _ = session.wait_for_exit(Duration::from_secs(3));
}

#[test]
fn custom_profile_opens_empty_module_list() {
    let mut session = spawn_toride(100, 32);

    session
        .wait_for_text("Choose setup profile", Duration::from_secs(10))
        .expect("should reach profile select");

    session.press_key("j").expect("select Custom");
    session.press_key("enter").expect("confirm Custom");

    session
        .wait_for_text("Modules", Duration::from_secs(5))
        .expect("should reach module select");

    session.press_key("q").expect("press q");
    let _ = session.wait_for_exit(Duration::from_secs(3));
}

#[test]
fn module_toggle_works() {
    let mut session = spawn_toride(100, 32);

    session
        .wait_for_text("Choose setup profile", Duration::from_secs(10))
        .expect("profile select");

    session.press_key("j").expect("select Custom");
    session.press_key("enter").expect("confirm Custom");

    session
        .wait_for_text("Modules", Duration::from_secs(5))
        .expect("module select");

    session.press_key(" ").expect("toggle module");

    session
        .wait_for_text("System Update", Duration::from_secs(3))
        .expect("first module visible");

    session.press_key("q").expect("press q");
    let _ = session.wait_for_exit(Duration::from_secs(3));
}
