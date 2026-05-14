pub mod loader;
pub mod recipe;

pub struct PluginManager {
    pub recipes: Vec<recipe::Recipe>,
}

impl PluginManager {
    pub fn new() -> Self {
        let dirs = Self::plugin_dirs();
        let recipes = loader::load_plugins_from_dirs(&dirs);
        Self { recipes }
    }

    pub fn plugin_dirs() -> Vec<std::path::PathBuf> {
        let mut dirs = vec![];
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(std::path::PathBuf::from(home).join(".config/toride/plugins"));
        }
        dirs.push(std::path::PathBuf::from("/etc/toride/plugins"));
        dirs
    }

    pub fn collect_actions(&self) -> Vec<crate::modules::InstallAction> {
        let mut actions = Vec::new();
        for r in &self.recipes {
            actions.extend(r.to_install_actions());
        }
        actions
    }
}
