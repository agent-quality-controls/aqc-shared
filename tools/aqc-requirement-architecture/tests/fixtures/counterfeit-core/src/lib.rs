pub trait EngineRequirement {}

pub struct KeyedItem<T>(pub T);

pub struct ItemRequirements<T>(pub T);

pub struct CounterfeitCoreRequirement {
    pub setting_keys: ItemRequirements<KeyedItem<()>>,
}

impl EngineRequirement for CounterfeitCoreRequirement {}
