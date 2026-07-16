pub trait EngineRequirement {}
pub trait AdapterRequirement {}

pub struct KeyedItem<T>(T);

pub struct ItemRequirements<T> {
    pub required: Vec<T>,
    pub forbidden: Vec<T>,
    pub exact: Option<Vec<T>>,
}

pub struct ExactFlagEngine {
    pub exact_settings: Option<String>,
}

impl EngineRequirement for ExactFlagEngine {}

pub struct ClosedFlagEngine {
    pub closed_settings: Option<String>,
}

impl EngineRequirement for ClosedFlagEngine {}

pub type ClosedSettingsAlias = bool;

pub struct AliasedFlagEngine {
    pub mode: ClosedSettingsAlias,
}

impl EngineRequirement for AliasedFlagEngine {}

pub struct RejectedAdapterRequirement {
    pub setting_keys: ItemRequirements<KeyedItem<()>>,
}

pub struct EngineRequirements {
    pub setting_keys: ItemRequirements<KeyedItem<()>>,
}

impl AdapterRequirement for RejectedAdapterRequirement {}

pub struct NoncanonicalMembershipField {
    pub membership: Option<ItemRequirements<KeyedItem<()>>>,
}

impl AdapterRequirement for NoncanonicalMembershipField {}

pub struct PrivateClosureField {
    exact_settings: bool,
}

impl EngineRequirement for PrivateClosureField {}

struct PrivateNestedClosure {
    exact_settings: bool,
}

pub struct PrivateNestedClosureRoot {
    child: PrivateNestedClosure,
}

impl EngineRequirement for PrivateNestedClosureRoot {}

mod ambiguous_one {
    pub struct Child {
        pub value: bool,
    }
}

mod ambiguous_two {
    pub struct Child {
        pub exact_settings: bool,
    }
}

pub struct AmbiguousChildRoot {
    child: ambiguous_one::Child,
}

impl EngineRequirement for AmbiguousChildRoot {}

pub use AdapterRequirement as IntermediateRequirementContract;
pub use IntermediateRequirementContract as RequirementContract;

pub struct HiddenAdapter {
    pub closed_settings: bool,
}

impl RequirementContract for HiddenAdapter {}

pub struct WrappedClosureEngine {
    pub closure: Option<ItemRequirements<KeyedItem<()>>>,
}

impl EngineRequirement for WrappedClosureEngine {}

pub struct TupleRequirementRoot(pub ExactFlagEngine);

impl EngineRequirement for TupleRequirementRoot {}

pub type AliasedRequirementRoot = ExactFlagEngine;

impl EngineRequirement for AliasedRequirementRoot {}

type Membership = ItemRequirements<KeyedItem<()>>;

pub fn direct_exact() -> Membership {
    Membership {
        required: Vec::new(),
        forbidden: Vec::new(),
        exact: Some(Vec::new()),
    }
}

pub fn nested_keys(values: &std::collections::BTreeMap<String, String>) -> Membership {
    Membership {
        required: Vec::new(),
        forbidden: Vec::new(),
        exact: Some(values.keys().map(|_| KeyedItem(())).collect::<Vec<_>>()),
    }
}

pub fn inferred_values(values: &std::collections::BTreeMap<String, String>) -> Membership {
    let inferred = values.values().map(|_| KeyedItem(())).collect::<Vec<_>>();
    Membership {
        required: Vec::new(),
        forbidden: Vec::new(),
        exact: Some(inferred),
    }
}

pub fn optional_filter(values: &[Option<String>]) -> Membership {
    let present = values
        .iter()
        .filter_map(Option::as_ref)
        .map(|_| KeyedItem(()))
        .collect::<Vec<_>>();
    Membership {
        required: Vec::new(),
        forbidden: Vec::new(),
        exact: Some(present),
    }
}

pub fn represented_discovery(values: &[String]) -> Membership {
    let represented_fields = values.iter().map(|_| KeyedItem(())).collect::<Vec<_>>();
    Membership {
        required: Vec::new(),
        forbidden: Vec::new(),
        exact: Some(represented_fields),
    }
}

pub fn assigned_exact(values: &std::collections::BTreeMap<String, String>) -> Membership {
    let mut requirements = direct_exact();
    requirements.exact = Some(values.keys().map(|_| KeyedItem(())).collect());
    requirements
}

pub fn renamed_local_membership_mutation(requirement: RejectedAdapterRequirement) -> Membership {
    let mut state = requirement.setting_keys;
    state.exact = Some(Vec::new());
    state
}

pub fn hidden_default_construction() -> RejectedAdapterRequirement {
    let x = ItemRequirements::default();
    RejectedAdapterRequirement { setting_keys: x }
}

pub fn cross_crate_membership_helper() -> RejectedAdapterRequirement {
    RejectedAdapterRequirement {
        setting_keys: external_membership::make(),
    }
}

pub fn cross_crate_wrapper_field() -> RejectedAdapterRequirement {
    RejectedAdapterRequirement {
        setting_keys: external_membership::wrapper().setting_keys,
    }
}

pub fn cross_crate_engine_membership_helper() -> EngineRequirements {
    EngineRequirements {
        setting_keys: external_membership::make(),
    }
}

pub fn rewrite_membership_parameter(value: &mut Membership) {
    value.exact = None;
}

pub fn replace_whole_membership(requirement: &mut RejectedAdapterRequirement) {
    requirement.setting_keys = ItemRequirements::default();
}

pub fn borrow_whole_membership(requirement: &mut RejectedAdapterRequirement) {
    let _ = std::mem::take(&mut requirement.setting_keys);
}

pub fn replace_dereferenced_membership(value: &mut Membership) {
    *value = Default::default();
}

pub fn borrow_dereferenced_membership(value: &mut Membership) {
    let _ = std::mem::take(&mut *value);
}

pub fn helper_returned_default() -> Membership {
    Default::default()
}

pub fn inferred_required(values: &std::collections::BTreeMap<String, String>) -> Membership {
    Membership {
        required: values.keys().map(|_| KeyedItem(())).collect(),
        forbidden: Vec::new(),
        exact: None,
    }
}

macro_rules! inferred_membership {
    ($values:expr) => {
        Membership {
            required: Vec::new(),
            forbidden: Vec::new(),
            exact: Some($values.keys().map(|_| KeyedItem(())).collect()),
        }
    };
}

pub fn inferred_by_macro(values: &std::collections::BTreeMap<String, String>) -> Membership {
    inferred_membership!(values)
}

pub fn inferred_by_extend(values: &std::collections::BTreeMap<String, String>) -> Membership {
    let mut membership = Membership::default();
    membership
        .required
        .extend(values.keys().map(|_| KeyedItem(())));
    membership
}

pub fn inferred_by_mutable_reference(
    values: &std::collections::BTreeMap<String, String>,
) -> Membership {
    let mut membership = Membership::default();
    let _ = std::mem::replace(
        &mut membership.exact,
        Some(values.keys().map(|_| KeyedItem(())).collect()),
    );
    membership
}

pub fn inferred_by_destructuring(
    values: &std::collections::BTreeMap<String, String>,
) -> Membership {
    let mut membership = Membership::default();
    let ItemRequirements { required, .. } = &mut membership;
    required.extend(values.keys().map(|_| KeyedItem(())));
    membership
}

pub fn default_membership_replacement() -> RejectedAdapterRequirement {
    let setting_keys = ItemRequirements::default();
    RejectedAdapterRequirement { setting_keys }
}

macro_rules! membership_default {
    () => {
        Membership::default()
    };
}

macro_rules! hidden_requirement_root {
    () => {
        pub struct MacroRequirementRoot;
        impl AdapterRequirement for MacroRequirementRoot {}
    };
}

hidden_requirement_root!();

pub fn local_macro_alias() -> Membership {
    membership_default!()
}

impl<T> Default for ItemRequirements<T> {
    fn default() -> Self {
        Self {
            required: Vec::new(),
            forbidden: Vec::new(),
            exact: None,
        }
    }
}
