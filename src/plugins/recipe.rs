use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub steps: Vec<RecipeStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeStep {
    #[serde(rename = "type")]
    pub step_type: String,
    #[serde(default)]
    pub packages: Vec<String>,
    pub cmd: Option<String>,
    pub args: Option<Vec<String>>,
    pub path: Option<String>,
    pub content: Option<String>,
    pub mode: Option<u32>,
    pub backup: Option<bool>,
    pub as_user: Option<String>,
}

impl Recipe {
    pub fn to_install_actions(&self) -> Vec<crate::modules::InstallAction> {
        use crate::modules::InstallAction;
        let mut actions = Vec::new();
        for step in &self.steps {
            match step.step_type.as_str() {
                "apt" => actions.push(InstallAction::AptInstall { packages: step.packages.clone() }),
                "dnf" => actions.push(InstallAction::DnfInstall { packages: step.packages.clone() }),
                "exec" => actions.push(InstallAction::Exec {
                    cmd: step.cmd.clone().unwrap_or_default(),
                    args: step.args.clone().unwrap_or_default(),
                    env: vec![],
                    as_user: step.as_user.clone(),
                }),
                "write" => {
                    if let (Some(path), Some(content)) = (&step.path, &step.content) {
                        actions.push(InstallAction::WriteFile {
                            path: path.clone(),
                            content: content.clone(),
                            mode: step.mode.unwrap_or(0o644),
                            backup: step.backup.unwrap_or(false),
                        });
                    }
                }
                _ => {}
            }
        }
        actions
    }
}

pub fn parse_recipe_file(path: &std::path::Path) -> Result<Recipe, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("read {:?}: {}", path, e))?;
    parse_recipe(&content)
}

pub fn parse_recipe(content: &str) -> Result<Recipe, String> {
    toml::from_str(content).map_err(|e| format!("parse recipe: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_recipe() {
        let toml = r#"
id = "my-stack"
name = "My Custom Stack"
version = "1.0"
description = "Custom module"

[[steps]]
type = "apt"
packages = ["htop", "curl"]
"#;
        let recipe = parse_recipe(toml).unwrap();
        assert_eq!(recipe.id, "my-stack");
        assert_eq!(recipe.steps.len(), 1);
        assert_eq!(recipe.steps[0].step_type, "apt");
        assert_eq!(recipe.steps[0].packages, vec!["htop", "curl"]);
    }

    #[test]
    fn parse_exec_step() {
        let toml = r#"
id = "test"
name = "Test"
version = "0.1"
description = "Test recipe"

[[steps]]
type = "exec"
cmd = "echo"
args = ["hello"]
"#;
        let recipe = parse_recipe(toml).unwrap();
        assert_eq!(recipe.steps[0].cmd.as_deref(), Some("echo"));
    }

    #[test]
    fn recipe_to_install_actions() {
        let toml = r#"
id = "test"
name = "Test"
version = "1.0"
description = "Test"

[[steps]]
type = "apt"
packages = ["vim"]

[[steps]]
type = "exec"
cmd = "echo"
args = ["done"]
"#;
        let recipe = parse_recipe(toml).unwrap();
        let actions = recipe.to_install_actions();
        assert_eq!(actions.len(), 2);
    }
}
