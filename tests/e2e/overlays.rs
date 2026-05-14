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
fn help_overlay_opens_and_closes() {
    let mut session = spawn_toride(100, 32);

    session
        .wait_for_text("Choose setup profile", Duration::from_secs(10))
        .expect("profile select");

    session.press_key("?").expect("open help");

    session
        .wait_for_text("Help", Duration::from_secs(3))
        .expect("help overlay should appear");

    session.press_key("esc").expect("close help");

    session
        .wait_for_text("Choose setup profile", Duration::from_secs(3))
        .expect("should return to profile select");

    session.press_key("q").expect("press q");
    let _ = session.wait_for_exit(Duration::from_secs(3));
}

#[test]
fn palette_overlay_opens_and_closes() {
    let mut session = spawn_toride(100, 32);

    session
        .wait_for_text("Choose setup profile", Duration::from_secs(10))
        .expect("profile select");

    session.press_key(":").expect("open palette");

    session
        .wait_for_text("Command Palette", Duration::from_secs(3))
        .expect("palette overlay should appear");

    session.press_key("esc").expect("close palette");

    session
        .wait_for_text("Choose setup profile", Duration::from_secs(3))
        .expect("should return to profile select");

    session.press_key("q").expect("press q");
    let _ = session.wait_for_exit(Duration::from_secs(3));
}

#[test]
fn search_overlay_opens() {
    let mut session = spawn_toride(100, 32);

    session
        .wait_for_text("Choose setup profile", Duration::from_secs(10))
        .expect("profile select");

    session.press_key("/").expect("open search");

    session
        .wait_for_stable_frame(Duration::from_millis(200), Duration::from_secs(3))
        .expect("frame should stabilize");

    session.press_key("esc").expect("close search");
    session.press_key("q").expect("press q");
    let _ = session.wait_for_exit(Duration::from_secs(3));
}
