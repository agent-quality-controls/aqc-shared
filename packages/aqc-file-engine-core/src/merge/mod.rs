//! Shared merge machinery for file-engine requirements.
//!
//! Adapters emit plain engine requirements tagged with provenance. Engine merge
//! code composes those plain requirements into resolved values, while keeping
//! the collected assertions needed for precise findings.

/// Forbidden-glob merge implementation.
mod forbidden_globs;
/// Resolved item membership data model.
mod item_model;
/// Item collection merge implementation.
mod items;
/// List-field merge implementation.
mod lists;
/// Merge data model and public aliases.
mod model;
/// Scalar and map merge implementation.
mod scalar;
/// Generic scalar assertion merge implementation.
mod scalar_assertion;

pub use forbidden_globs::resolve_forbidden_globs;
pub use item_model::{
    FileItemRequirement, ItemPresenceDifference, ResolvedAllowedItems, ResolvedExactItems,
    ResolvedItemMembership, ResolvedItemRequirements,
};
pub use items::{
    asserted_items, compose_item_by, item_presence_difference, resolve_items,
    resolve_key_membership,
};
pub use lists::{
    apply_list_requirements, exact_list_difference, render_list_requirement, resolve_exact_list,
    resolve_list,
};
pub(crate) use model::sort_provenanced;
pub use model::{
    AllowedItems, AllowedItemsInput, Collected, ConflictEntry, Contributor, Contributors,
    ExactInput, ExactItems, ExactItemsInput, ExactListDifference, FileKeyRequirement,
    ForbiddenGlobRequirement, ForbiddenGlobRequirements, ForbiddenItemMap, ForbiddenItemResolution,
    GlobAssertion, GlobAssertionGroups, GlobAssertionInput, GlobInput, GlobResolutionMap,
    GroupedAssertions, ItemAssertion, ItemAssertionGroups, ItemAssertionInput, ItemInput,
    ItemRequirementMap, ItemRequirements, KeyedItem, KeyedValueMap, ListExact, ListInput,
    ListRequirements, MapInput, MapInputs, MemberInputs, MessagePair, OptionalInput, Provenanced,
    RequiredItemResolution, Resolve, ResolvedAssertion, ResolvedAssertionOption, ResolvedExactList,
    ResolvedForbiddenGlobRequirements, ResolvedListRequirements, ResolvedMap, ResolvedRequirement,
    ResolvedSame, ResolvedSameOption, ResolvedStringMembers, ScalarAssertion, ScalarOperation,
    ScalarValue, VersionFloor, resolved_map_attribution,
};
pub use scalar::{
    compose_optional_field, compose_string_list, compose_string_set, keyed_entries_eq,
    push_conflict, push_rendered_conflict, resolve_all_equal, resolve_map, resolve_maybe,
    resolve_scalar, strongest_version_floor,
};
pub use scalar_assertion::{
    render_scalar_assertion, scalar_assertion_matches, scalar_assertion_writable_value,
};
