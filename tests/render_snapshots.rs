use ratatui::backend::TestBackend;
use ratatui::Terminal;
use toride::tui::caps::TerminalCaps;
use toride::tui::model::*;
use toride::tui::view::view;

fn render_model(model: &Model, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| view(frame, &model)).unwrap();
    format!("{:?}", terminal.backend())
}

#[test]
fn welcome_screen_at_100x32() {
    let mut model = Model::initial_for_test();
    model.screen_stack = vec![Screen::Welcome];
    model.system.os_name = "Ubuntu 24.04 LTS".into();
    model.system.current_user = "root".into();
    model.system.is_root = true;
    let buffer = render_model(&model, 100, 32);
    assert!(buffer.contains("Toride"));
    assert!(buffer.contains("Ubuntu 24.04 LTS"));
}

#[test]
fn profile_select_screen_at_100x32() {
    let mut model = Model::initial_for_test();
    model.screen_stack = vec![Screen::ProfileSelect];
    model.profile = Some(Profile::Basic);
    let buffer = render_model(&model, 100, 32);
    assert!(buffer.contains("Basic"));
    assert!(buffer.contains("Custom"));
}

#[test]
fn module_select_screen_at_100x32() {
    let mut model = Model::initial_for_test();
    model.screen_stack = vec![Screen::ModuleSelect];
    model.selection.toggle(ModuleId::Docker);
    let buffer = render_model(&model, 100, 32);
    assert!(buffer.contains("Docker"));
    assert!(buffer.contains("Modules"));
}

#[test]
fn module_select_screen_at_80x24() {
    let mut model = Model::initial_for_test();
    model.screen_stack = vec![Screen::ModuleSelect];
    let buffer = render_model(&model, 80, 24);
    // should render without panic, even in narrow mode
    assert!(!buffer.is_empty());
}

#[test]
fn too_small_terminal_shows_message() {
    let model = Model::initial_for_test();
    let buffer = render_model(&model, 60, 20);
    assert!(buffer.contains("too small"));
}

#[test]
fn summary_screen() {
    let mut model = Model::initial_for_test();
    model.screen_stack = vec![Screen::Summary];
    model.selection.toggle(ModuleId::Docker);
    model.selection.toggle(ModuleId::Ufw);
    let buffer = render_model(&model, 100, 32);
    assert!(buffer.contains("Docker"));
    assert!(buffer.contains("UFW"));
}
