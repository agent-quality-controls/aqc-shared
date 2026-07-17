use duplicate_facade::{ItemRequirements, KeyedItem};

pub trait EngineRequirement {}

pub struct DuplicateFacadeRequirement {
    pub setting_keys: ItemRequirements<KeyedItem<()>>,
}

impl EngineRequirement for DuplicateFacadeRequirement {}
