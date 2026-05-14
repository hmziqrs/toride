pub mod basic;
pub mod custom;
pub mod sandbox;

use crate::tui::model::{ModuleId, Profile};

pub fn profile_defaults(profile: Profile) -> Vec<ModuleId> {
    match profile {
        Profile::Basic => basic::defaults(),
        Profile::Sandbox => sandbox::defaults(),
        Profile::Custom => custom::defaults(),
    }
}
