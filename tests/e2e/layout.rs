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
fn renders_at_80x24() {
    let mut session = spawn_toride(80, 24);

    session
        .wait_for_text("Toride", Duration::from_secs(10))
        .expect("should show Toride at 80x24");

    session
        .wait_for_text("Choose setup profile", Duration::from_secs(10))
        .expect("should reach profile select at 80x24");

    session.press_key("q").expect("press q");
    let _ = session.wait_for_exit(Duration::from_secs(3));
}

#[test]
fn renders_at_100x32() {
    let mut session = spawn_toride(100, 32);

    session
        .wait_for_text("Choose setup profile", Duration::from_secs(10))
        .expect("should reach profile select at 100x32");

    session.press_key("q").expect("press q");
    let _ = session.wait_for_exit(Duration::from_secs(3));
}

#[test]
fn renders_at_140x40() {
    let mut session = spawn_toride(140, 40);

    session
        .wait_for_text("Choose setup profile", Duration::from_secs(10))
        .expect("should reach profile select at 140x40");

    session.press_key("q").expect("press q");
    let _ = session.wait_for_exit(Duration::from_secs(3));
}

#[test]
fn too_small_shows_warning() {
    let mut session = spawn_toride(60, 20);

    // wait_for_text may not parse correctly at small PTY sizes,
    // so wait for a stable frame then check manually
    session
        .wait_for_stable_frame(Duration::from_millis(300), Duration::from_secs(5))
        .expect("frame should stabilize");

    let frame = session.capture_frame();
    let text = frame.all_text();
    assert!(
        text.contains("Terminal too small"),
        "should show too-small warning at 60x20, got:\n{}",
        text
    );

    session.press_key("q").expect("press q");
    let _ = session.wait_for_exit(Duration::from_secs(3));
}
