pub use aqc_file_engine_core::{EngineRequirement, ItemRequirements, KeyedItem};

pub struct ImportedEngineRequirements {
    pub setting_keys: ItemRequirements<KeyedItem<()>>,
}

impl EngineRequirement for ImportedEngineRequirements {}
