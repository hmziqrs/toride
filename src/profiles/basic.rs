use crate::tui::model::ModuleId;

pub fn defaults() -> Vec<ModuleId> {
    vec![
        ModuleId::SystemUpdate,
        ModuleId::Swap,
        ModuleId::UserSsh,
        ModuleId::Ufw,
        ModuleId::Docker,
    ]
}
