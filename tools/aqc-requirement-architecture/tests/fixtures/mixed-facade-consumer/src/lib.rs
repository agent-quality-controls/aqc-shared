pub trait EngineRequirement {}

pub struct CounterfeitNestedRequirement {
    pub setting_keys:
        mixed_facade::counterfeit::ItemRequirements<mixed_facade::counterfeit::KeyedItem<()>>,
}

impl EngineRequirement for CounterfeitNestedRequirement {}
