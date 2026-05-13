use toride::tui::forms::validators;
use toride::tui::model::*;
use toride::profiles;

#[test]
fn basic_profile_selects_expected_modules() {
    let defaults = profiles::profile_defaults(Profile::Basic);
    assert!(defaults.contains(&ModuleId::SystemUpdate));
    assert!(defaults.contains(&ModuleId::Swap));
    assert!(defaults.contains(&ModuleId::UserSsh));
    assert!(defaults.contains(&ModuleId::Ufw));
    assert!(defaults.contains(&ModuleId::Docker));
}

#[test]
fn custom_profile_selects_nothing() {
    let defaults = profiles::profile_defaults(Profile::Custom);
    assert!(defaults.is_empty());
}

#[test]
fn toggle_module_selection() {
    let mut state = SelectionState::new();
    assert!(!state.modules[&ModuleId::Docker].selected);
    state.toggle(ModuleId::Docker);
    assert!(state.modules[&ModuleId::Docker].selected);
    state.toggle(ModuleId::Docker);
    assert!(!state.modules[&ModuleId::Docker].selected);
}

#[test]
fn select_all_selects_everything() {
    let mut state = SelectionState::new();
    state.select_all();
    for m in state.modules.values() {
        assert!(m.selected);
    }
}

#[test]
fn select_none_deselects_everything() {
    let mut state = SelectionState::new();
    state.select_all();
    state.select_none();
    for m in state.modules.values() {
        assert!(!m.selected);
    }
}

#[test]
fn invert_selection() {
    let mut state = SelectionState::new();
    state.toggle(ModuleId::Docker);
    state.invert();
    assert!(!state.modules[&ModuleId::Docker].selected);
    assert!(state.modules[&ModuleId::SystemUpdate].selected);
}

#[test]
fn set_from_profile_overrides() {
    let mut state = SelectionState::new();
    state.select_all();
    let defaults = profiles::profile_defaults(Profile::Basic);
    state.set_from_profile(&defaults);
    assert!(state.modules[&ModuleId::Docker].selected);
    assert!(!state.modules[&ModuleId::Mise].selected);
}

#[test]
fn selected_ids_returns_only_selected() {
    let mut state = SelectionState::new();
    state.toggle(ModuleId::Docker);
    state.toggle(ModuleId::Ufw);
    let ids = state.selected_ids();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&ModuleId::Docker));
    assert!(ids.contains(&ModuleId::Ufw));
}

#[test]
fn username_validator_rejects_root() {
    assert!(validators::username("root").is_err());
}

#[test]
fn username_validator_rejects_empty() {
    assert!(validators::username("").is_err());
}

#[test]
fn username_validator_accepts_valid() {
    assert!(validators::username("deploy").is_ok());
    assert!(validators::username("_admin").is_ok());
    assert!(validators::username("user-123").is_ok());
}

#[test]
fn username_validator_rejects_numeric_start() {
    assert!(validators::username("1user").is_err());
}

#[test]
fn ssh_key_validator_rejects_empty() {
    assert!(validators::ssh_public_key("").is_err());
}

#[test]
fn ssh_key_validator_rejects_private_key() {
    assert!(validators::ssh_public_key("-----BEGIN OPENSSH PRIVATE KEY-----").is_err());
}

#[test]
fn ssh_key_validator_accepts_valid() {
    assert!(validators::ssh_public_key("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI test@host").is_ok());
    assert!(validators::ssh_public_key("ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQC test").is_ok());
}

#[test]
fn ssh_key_validator_rejects_bad_format() {
    assert!(validators::ssh_public_key("not-a-key").is_err());
}

#[test]
fn swap_size_validator_accepts_valid() {
    assert!(validators::swap_size("2G").is_ok());
    assert!(validators::swap_size("512M").is_ok());
    assert!(validators::swap_size("0").is_ok());
    assert!(validators::swap_size("").is_ok());
}

#[test]
fn swap_size_validator_rejects_bad_format() {
    assert!(validators::swap_size("abc").is_err());
    assert!(validators::swap_size("2T").is_err());
}

#[test]
fn port_validator_accepts_valid() {
    assert!(validators::port("22").is_ok());
    assert!(validators::port("8080").is_ok());
    assert!(validators::port("65535").is_ok());
}

#[test]
fn port_validator_rejects_invalid() {
    assert!(validators::port("abc").is_err());
    assert!(validators::port("0").is_err());
}

#[test]
fn hostname_validator_accepts_valid() {
    assert!(validators::hostname("my-server").is_ok());
    assert!(validators::hostname("web01").is_ok());
}

#[test]
fn hostname_validator_rejects_hyphen_edges() {
    assert!(validators::hostname("-server").is_err());
    assert!(validators::hostname("server-").is_err());
}

#[test]
fn ring_buffer_push_and_iter() {
    let mut buf: RingBuffer<i32> = RingBuffer::new(3);
    buf.push(1);
    buf.push(2);
    buf.push(3);
    assert_eq!(buf.len(), 3);
    buf.push(4);
    assert_eq!(buf.len(), 3);
    let vals: Vec<&i32> = buf.iter().collect();
    assert_eq!(vals, vec![&2, &3, &4]);
}

#[test]
fn module_id_label_matches() {
    assert_eq!(ModuleId::SystemUpdate.label(), "System Update");
    assert_eq!(ModuleId::Docker.label(), "Docker");
    assert_eq!(ModuleId::Mise.label(), "Language Runtimes (mise)");
}

#[test]
fn module_categories() {
    assert_eq!(ModuleId::SystemUpdate.category(), Category::SystemBasics);
    assert_eq!(ModuleId::UserSsh.category(), Category::UsersAndSsh);
    assert_eq!(ModuleId::Ufw.category(), Category::FirewallAndSecurity);
    assert_eq!(ModuleId::Docker.category(), Category::Containers);
    assert_eq!(ModuleId::Mise.category(), Category::DeveloperRuntimes);
}

#[test]
fn screen_overlay_detection() {
    assert!(Screen::Help.is_overlay());
    assert!(Screen::Palette.is_overlay());
    assert!(Screen::Search.is_overlay());
    assert!(!Screen::Welcome.is_overlay());
    assert!(!Screen::ModuleSelect.is_overlay());
}

#[test]
fn toast_lifecycle() {
    let mut model = Model::initial_for_test();
    model.add_toast("info msg".into(), ToastKind::Info);
    assert_eq!(model.toasts.len(), 1);
    model.add_toast("error msg".into(), ToastKind::Error);
    assert_eq!(model.toasts.len(), 2);
    model.dismiss_toast();
    assert_eq!(model.toasts.len(), 1);
}

#[test]
fn screen_stack_navigation() {
    let mut model = Model::initial_for_test();
    assert_eq!(*model.current_screen(), Screen::Welcome);
    model.push_screen(Screen::ProfileSelect);
    assert_eq!(*model.current_screen(), Screen::ProfileSelect);
    model.pop_screen();
    assert_eq!(*model.current_screen(), Screen::Welcome);
}

#[test]
fn install_action_shell_preview() {
    use toride::modules::InstallAction;
    let action = InstallAction::AptInstall {
        packages: vec!["docker-ce".into(), "docker-compose-plugin".into()],
    };
    assert!(action.to_shell_preview().contains("docker-ce"));
    assert!(action.to_shell_preview().contains("apt install"));
}
