use aqc_file_engine_core::{ItemRequirements, KeyedItem};

pub trait EngineRequirement {}

pub struct CounterfeitDependencyRequirement {
    pub setting_keys: ItemRequirements<KeyedItem<()>>,
}

impl EngineRequirement for CounterfeitDependencyRequirement {}
