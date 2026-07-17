//! Shared framework types for `aqc-{domain}-engine` crates.
//!
//! See the architecture plan:
//! `guardrail3/.plans/g3v2-architecture/2026-05-21-195830-repo-workspace-plugin-generation-model.md`.
//!
//! Every type in this crate is pure data; this crate performs zero I/O.

#[cfg(feature = "api")]
pub mod contract;
#[cfg(feature = "api")]
pub mod engine;
#[cfg(feature = "api")]
pub mod finding;
#[cfg(feature = "api")]
pub mod merge;
#[cfg(feature = "api")]
pub mod requirement;
#[cfg(feature = "api")]
pub mod types;
#[cfg(feature = "api")]
pub mod version;

#[cfg(feature = "api")]
pub use contract::{ContractViolation, check_from_empty};
#[cfg(feature = "api")]
pub use engine::{Engine, FileEngine, merged_reconcile};
#[cfg(feature = "api")]
pub use finding::{Finding, FindingKey};
#[cfg(feature = "api")]
pub use merge::{
    AllowedItems, ConflictEntry, ExactListDifference, FileItemRequirement, FileKeyRequirement,
    ForbiddenGlobRequirement, ForbiddenGlobRequirements, ItemAssertionInput,
    ItemPresenceDifference, ItemRequirements, KeyedItem, ListRequirements, RequiredItemResolution,
    Resolve, ResolvedAllowedItems, ResolvedExactItems, ResolvedExactList,
    ResolvedForbiddenGlobRequirements, ResolvedItemMembership, ResolvedItemRequirements,
    ResolvedListRequirements, ResolvedRequirement, ScalarAssertion, ScalarOperation, ScalarValue,
    apply_list_requirements, asserted_items, compose_item_by, compose_optional_field,
    compose_string_list, compose_string_set, exact_list_difference, item_presence_difference,
    keyed_entries_eq, push_conflict, push_rendered_conflict, render_list_requirement,
    render_scalar_assertion, resolve_all_equal, resolve_exact_list, resolve_forbidden_globs,
    resolve_items, resolve_key_membership, resolve_list, resolve_map, resolve_maybe,
    resolve_scalar, resolved_map_attribution, scalar_assertion_matches,
    scalar_assertion_writable_value, strongest_version_floor,
};
#[cfg(feature = "api")]
pub use requirement::EngineRequirement;
#[cfg(feature = "api")]
pub use types::{
    ConfigScalar, DottedVersion, EngineOutput, OnEmpty, OnEmptyClass, Provenance, Severity,
};
#[cfg(feature = "api")]
pub use version::parse_version_tuple;
