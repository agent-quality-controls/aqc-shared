pub trait AdapterRequirement {}

use fixture_engine::{ItemRequirements, KeyedItem};

pub struct AcceptedAdapterRequirement {
    pub setting_keys: ItemRequirements<KeyedItem<()>>,
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

pub struct UnrelatedKeysEngine {
    pub cache_keys: Vec<String>,
}

pub trait EngineRequirement {}

impl EngineRequirement for UnrelatedKeysEngine {}

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

pub fn lower_direct(requirement: AcceptedAdapterRequirement) -> EngineRequirements {
    EngineRequirements {
        contents: requirement.setting_keys,
    }
}

pub fn lower_through_typed_local(requirement: AcceptedAdapterRequirement) -> EngineRequirements {
    let contents: ItemRequirements<KeyedItem<()>> = requirement.setting_keys;
    EngineRequirements { contents }
}

pub fn lower_with_neutral_engine_field() -> EngineRequirements {
    EngineRequirements {
        contents: ItemRequirements::default(),
    }
}

pub struct UnrelatedSettingKeys {
    pub setting_keys: Vec<String>,
}

pub fn unrelated_destructuring(value: UnrelatedSettingKeys) {
    let UnrelatedSettingKeys { setting_keys } = value;
    drop(setting_keys);
}

pub fn unrelated_construction_and_mutation() {
    let mut value = UnrelatedSettingKeys {
        setting_keys: vec!["ordinary-data".to_owned()],
    };
    value.setting_keys.push("more-data".to_owned());
}

mod qualified_names {
    pub struct AcceptedAdapterRequirement {
        pub setting_keys: Vec<String>,
    }

    pub struct Settings {
        pub setting_keys: Vec<String>,
    }
}

pub fn unrelated_qualified_adapter_name(
    value: qualified_names::AcceptedAdapterRequirement,
) -> EngineRequirements {
    drop(value);
    EngineRequirements {
        contents: ItemRequirements::default(),
    }
}

pub fn unrelated_qualified_same_name_construction() {
    let mut value = qualified_names::Settings {
        setting_keys: vec!["ordinary-data".to_owned()],
    };
    value.setting_keys.push("more-data".to_owned());
}

mod local_adapter_shadow {
    use super::{EngineRequirements, ItemRequirements};

    pub struct AcceptedAdapterRequirement {
        pub setting_keys: Vec<String>,
    }

    pub fn independent_default(_value: AcceptedAdapterRequirement) -> EngineRequirements {
        EngineRequirements {
            contents: ItemRequirements::default(),
        }
    }
}

mod scoped_membership_alias {
    use super::{EngineRequirement, ItemRequirements, KeyedItem};

    type Membership = ItemRequirements<KeyedItem<()>>;

    pub struct ScopedAliasEngine {
        pub setting_keys: Membership,
    }

    impl EngineRequirement for ScopedAliasEngine {}
}

mod unrelated_alias_collision {
    type Membership = Vec<String>;

    pub fn ordinary_values() -> Membership {
        Vec::new()
    }
}

mod imported_membership_definition {
    use super::{ItemRequirements, KeyedItem};

    pub type ImportedMembership = ItemRequirements<KeyedItem<()>>;
}

mod imported_membership_use {
    use super::{EngineRequirement, imported_membership_definition::ImportedMembership};

    pub struct ImportedAliasEngine {
        pub setting_keys: ImportedMembership,
    }

    impl EngineRequirement for ImportedAliasEngine {}
}

pub fn generic_type_shadow<AcceptedAdapterRequirement>(
    _value: AcceptedAdapterRequirement,
) -> EngineRequirements {
    EngineRequirements {
        contents: ItemRequirements::default(),
    }
}

mod unrelated_output_shadow {
    pub struct EngineRequirements;

    fn helper() -> EngineRequirements {
        EngineRequirements
    }

    pub fn returns_unrelated_output() -> EngineRequirements {
        helper()
    }
}
