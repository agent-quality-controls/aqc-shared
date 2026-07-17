pub trait EngineRequirement {}
pub trait AdapterRequirement {}

use aqc_file_engine_core::{ItemRequirements, KeyedItem};
use aqc_file_engine_core::ItemRequirements as RenamedItemRequirements;

mod counterfeit {
    pub struct KeyedItem<T>(pub T);

    pub struct ItemRequirements<T>(pub T);
}

pub struct CounterfeitEngine {
    pub setting_keys: counterfeit::ItemRequirements<counterfeit::KeyedItem<()>>,
}

impl EngineRequirement for CounterfeitEngine {}

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

impl EngineRequirement for EngineRequirements {}

pub struct MembershipSmuggler {
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

pub fn assigned_allowed(values: &std::collections::BTreeMap<String, String>) -> Membership {
    let mut requirements = ItemRequirements::default();
    requirements.allowed = Some(values.keys().map(|_| KeyedItem(())).collect());
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

pub fn local_engine_membership_helper() -> EngineRequirements {
    EngineRequirements {
        setting_keys: direct_exact(),
    }
}

pub fn type_annotated_membership_local_is_tracked() -> EngineRequirements {
    let mut setting_keys: Membership = external_membership::make();
    setting_keys.exact = None;
    EngineRequirements { setting_keys }
}

pub fn shadowed_membership_transfer(requirement: RejectedAdapterRequirement) -> EngineRequirements {
    let setting_keys = requirement.setting_keys;
    let setting_keys = external_membership::make();
    EngineRequirements { setting_keys }
}

pub fn tuple_shadowed_membership_transfer(
    requirement: RejectedAdapterRequirement,
) -> EngineRequirements {
    let setting_keys = requirement.setting_keys;
    let (setting_keys,) = (external_membership::make(),);
    EngineRequirements { setting_keys }
}

pub fn rewrite_membership_parameter(value: &mut Membership) {
    value.exact = None;
}

pub fn membership_helper_parameter(value: Membership) -> EngineRequirements {
    EngineRequirements { setting_keys: value }
}

pub fn discard_policy_membership(
    _requirement: RejectedAdapterRequirement,
) -> EngineRequirements {
    EngineRequirements {
        setting_keys: ItemRequirements::default(),
    }
}

pub fn same_name_destructuring_smuggle(value: MembershipSmuggler) -> EngineRequirements {
    let MembershipSmuggler { setting_keys } = value;
    EngineRequirements { setting_keys }
}

pub fn local_whole_engine_helper() -> EngineRequirements {
    make_engine()
}

pub fn cross_crate_whole_engine_helper() -> external_membership::ExternalEngineRequirements {
    external_membership::make_engine()
}

pub fn local_bound_whole_engine_helper() -> EngineRequirements {
    let result = make_engine();
    result
}

pub fn conditional_whole_engine_helper(flag: bool) -> EngineRequirements {
    if flag { make_engine() } else { make_engine() }
}

pub fn tuple_whole_engine_helper() -> EngineRequirements {
    let (result,) = (make_engine(),);
    result
}

pub fn reassigned_whole_engine_helper(
    requirement: RejectedAdapterRequirement,
) -> EngineRequirements {
    let mut result = EngineRequirements {
        setting_keys: requirement.setting_keys,
    };
    result = make_engine();
    result
}

pub fn closure_membership_parameter(requirement: RejectedAdapterRequirement) -> EngineRequirements {
    let mutate = |value: &mut Membership| {
        value.exact = None;
    };
    let _ = mutate;
    EngineRequirements {
        setting_keys: requirement.setting_keys,
    }
}

pub fn inferred_closure_membership(requirement: RejectedAdapterRequirement) -> EngineRequirements {
    let consume = |value| drop(value);
    let alias = consume;
    alias(requirement.setting_keys.clone());
    EngineRequirements {
        setting_keys: requirement.setting_keys,
    }
}

pub fn tuple_closure_alias_membership(
    requirement: RejectedAdapterRequirement,
) -> EngineRequirements {
    let take = |value| drop(value);
    let (alias,) = (take,);
    alias(requirement.setting_keys.clone());
    EngineRequirements {
        setting_keys: requirement.setting_keys,
    }
}

pub fn referenced_closure_alias_membership(
    requirement: RejectedAdapterRequirement,
) -> EngineRequirements {
    let take = |value| drop(value);
    let alias = &take;
    alias(requirement.setting_keys.clone());
    EngineRequirements {
        setting_keys: requirement.setting_keys,
    }
}

mod qualified_local_root {
    use super::{AdapterRequirement, ItemRequirements, KeyedItem};

    pub struct NestedAdapterRequirement {
        pub setting_keys: ItemRequirements<KeyedItem<()>>,
    }

    impl AdapterRequirement for NestedAdapterRequirement {}
}

mod glob_adapter_root {
    use super::{AdapterRequirement, ItemRequirements, KeyedItem};

    pub struct GlobAdapterRequirement {
        pub setting_keys: ItemRequirements<KeyedItem<()>>,
    }

    impl AdapterRequirement for GlobAdapterRequirement {}
}

mod glob_import_consumer {
    use super::glob_adapter_root::*;
    use super::{EngineRequirements, ItemRequirements};

    pub fn glob_import_discard(_requirement: GlobAdapterRequirement) -> EngineRequirements {
        EngineRequirements {
            setting_keys: ItemRequirements::default(),
        }
    }
}

mod block_import_consumer {
    use super::qualified_local_root::NestedAdapterRequirement as Input;
    use super::{EngineRequirements, ItemRequirements};

    pub fn block_import_noise() {
        use super::super::impostor::RejectedAdapterRequirement as Input;
        let _ = std::mem::size_of::<Input>();
    }

    pub fn block_import_discard(_requirement: Input) -> EngineRequirements {
        EngineRequirements {
            setting_keys: ItemRequirements::default(),
        }
    }
}

use qualified_local_root::NestedAdapterRequirement as ImportedAdapterInput;

pub fn imported_adapter_alias_discard(_requirement: ImportedAdapterInput) -> EngineRequirements {
    EngineRequirements {
        setting_keys: ItemRequirements::default(),
    }
}

pub fn qualified_local_adapter_discard(
    _requirement: qualified_local_root::NestedAdapterRequirement,
) -> EngineRequirements {
    EngineRequirements {
        setting_keys: ItemRequirements::default(),
    }
}

struct One(EngineRequirements);

pub fn tuple_struct_whole_engine_helper() -> EngineRequirements {
    let One(result) = One(make_engine());
    result
}

mod parent_trait_alias {
    use super::{EngineRequirement, ItemRequirements, KeyedItem};
    use EngineRequirement as Contract;

    pub mod nested {
        use super::{ItemRequirements, KeyedItem};
        use super::Contract as NestedContract;

        pub struct ParentAliasedEngine {
            pub setting_keys: ItemRequirements<KeyedItem<()>>,
        }

        impl super::Contract for ParentAliasedEngine {}

        pub struct ChainedAliasedEngine {
            pub setting_keys: ItemRequirements<KeyedItem<()>>,
        }

        impl NestedContract for ChainedAliasedEngine {}
    }
}

pub fn destructured_policy_membership_discard(
    RejectedAdapterRequirement { setting_keys: _ }: RejectedAdapterRequirement,
) -> EngineRequirements {
    EngineRequirements {
        setting_keys: ItemRequirements::default(),
    }
}

fn make_engine() -> EngineRequirements {
    EngineRequirements {
        setting_keys: ItemRequirements::default(),
    }
}

impl RejectedAdapterRequirement {
    pub fn discard_through_self(self) -> EngineRequirements {
        EngineRequirements {
            setting_keys: ItemRequirements::default(),
        }
    }
}

mod impostor {
    use super::{ItemRequirements, KeyedItem};

    pub struct RejectedAdapterRequirement {
        pub setting_keys: ItemRequirements<KeyedItem<()>>,
    }
}

pub fn qualified_same_name_destructuring(
    value: impostor::RejectedAdapterRequirement,
) -> EngineRequirements {
    let impostor::RejectedAdapterRequirement { setting_keys } = value;
    EngineRequirements { setting_keys }
}

mod alias_one {
    use super::EngineRequirement as RequirementContract;

    pub struct AliasedEngine;
    impl RequirementContract for AliasedEngine {}
}

mod alias_two {
    pub trait OtherRequirement {}
    use OtherRequirement as RequirementContract;

    pub struct Other;
    impl RequirementContract for Other {}
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
