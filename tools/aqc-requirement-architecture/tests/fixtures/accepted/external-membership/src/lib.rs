use aqc_file_engine_core::{ItemRequirements, KeyedItem};

pub trait EngineRequirement {}

pub struct ExternalEngineRequirements {
    pub setting_keys: ItemRequirements<KeyedItem<()>>,
}

impl EngineRequirement for ExternalEngineRequirements {}

pub fn make() -> ItemRequirements<KeyedItem<()>> {
    ItemRequirements::default()
}

pub fn make_engine() -> ExternalEngineRequirements {
    ExternalEngineRequirements {
        setting_keys: ItemRequirements::default(),
    }
}
