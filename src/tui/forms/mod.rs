pub mod validators;

use crate::tui::model::FormField;
use std::collections::HashMap;

pub type Validator = fn(&str) -> Result<(), String>;

pub fn validators() -> HashMap<FormField, Validator> {
    let mut m = HashMap::new();
    m.insert(FormField::Username, validators::username as Validator);
    m.insert(FormField::SshPublicKey, validators::ssh_public_key as Validator);
    m.insert(FormField::SwapSize, validators::swap_size as Validator);
    m.insert(FormField::SshPort, validators::port as Validator);
    m
}
