pub trait EngineRequirement {}
pub trait AdapterRequirement {}

use aqc_file_engine_core::{ItemRequirements, KeyedItem};

pub struct InventoryEngineRequirement {
    pub root_keys: ItemRequirements<KeyedItem<()>>,
}

impl EngineRequirement for InventoryEngineRequirement {}

pub type ExplicitKeys = ItemRequirements<KeyedItem<()>>;

pub struct InventoryAdapterRequirement {
    pub setting_keys: ExplicitKeys,
}

impl AdapterRequirement for InventoryAdapterRequirement {}
