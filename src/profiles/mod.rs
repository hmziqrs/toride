pub mod basic;
pub mod custom;

use crate::tui::model::{ModuleId, Profile};

pub fn profile_defaults(profile: Profile) -> Vec<ModuleId> {
    match profile {
        Profile::Basic => basic::defaults(),
        Profile::Custom => custom::defaults(),
    }
}
