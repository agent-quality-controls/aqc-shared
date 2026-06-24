//! Shared merge machinery for file-engine requirements.
//!
//! Adapters emit plain engine requirements tagged with provenance. Engine merge
//! code composes those plain requirements into resolved values, while keeping
//! the collected assertions needed for precise findings.

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

pub use items::{compose_item_by, resolve_forbidden_globs, resolve_items};
pub use lists::{resolve_exact_list, resolve_list};
pub use model::{
    ClosedInput, Collected, ConflictEntry, Contributor, Contributors, ExactInput,
    FileItemRequirement, ForbiddenGlobRequirement, ForbiddenGlobRequirements, ForbiddenItemMap,
    ForbiddenItemResolution, GlobAssertion, GlobAssertionGroups, GlobAssertionInput, GlobInput,
    GlobResolutionMap, GroupedAssertions, ItemAssertion, ItemAssertionGroups, ItemAssertionInput,
    ItemInput, ItemRequirementMap, ItemRequirements, KeyedItem, KeyedValueMap, ListExact,
    ListInput, ListRequirements, MapInput, MapInputs, MemberInputs, MessagePair, OptionalInput,
    Provenanced, RequiredItemResolution, Resolve, ResolvedAssertion, ResolvedAssertionOption,
    ResolvedExactList, ResolvedForbiddenGlobRequirements, ResolvedItemRequirements,
    ResolvedListRequirements, ResolvedMap, ResolvedRequirement, ResolvedSame, ResolvedSameOption,
    ResolvedStringMembers, ScalarAssertion, ScalarOperation, ScalarValue, VersionFloor,
};
pub use scalar::{
    compose_optional_field, compose_string_list, compose_string_set, keyed_entries_eq,
    resolve_all_equal, resolve_map, resolve_maybe, resolve_scalar, strongest_version_floor,
};
