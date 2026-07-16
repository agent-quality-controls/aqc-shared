pub trait AdapterRequirement {}

use aqc_file_engine_core::{ItemRequirements, KeyedItem};

use crate::ItemRequirements as Membership;

pub struct AcceptedAdapterRequirement {
    pub setting_keys: Membership<KeyedItem<()>>,
}

pub struct UnrelatedRule {
    pub required: Vec<String>,
}

pub struct UnrelatedRequirement {
    pub rule: UnrelatedRule,
}

pub struct EngineRequirements {
    pub contents: ItemRequirements<KeyedItem<()>>,
}

pub struct UnrelatedCache {
    pub cache_keys: Vec<String>,
}

pub fn unrelated_required_field_mutation(mut rule: UnrelatedRule) {
    rule.required.sort();
}

pub fn unrelated_nested_required_field_mutation(
    requirement: &mut UnrelatedRequirement,
    value: String,
) {
    requirement.rule.required.push(value);
}

pub fn unrelated_cache_key_mutation(cache: &mut UnrelatedCache) {
    cache.cache_keys.sort();
}

pub fn unrelated_membership_words_macro() {
    macro_rules! audit {
        ($($token:ident)*) => {};
    }
    audit!(required forbidden exact);
}

pub fn unrelated_external_macro() -> u8 {
    std::thread_local! {
        static VALUE: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
    }
    VALUE.get()
}

impl AdapterRequirement for AcceptedAdapterRequirement {}

pub fn lower(requirement: AcceptedAdapterRequirement) -> ItemRequirements<KeyedItem<String>> {
    requirement.setting_keys.map(|_| KeyedItem("file-key".to_owned()))
}

pub fn lower_with_neutral_engine_field() -> EngineRequirements {
    EngineRequirements {
        contents: ItemRequirements::default(),
    }
}
