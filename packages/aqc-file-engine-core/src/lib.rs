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
pub mod toml_helpers;
#[cfg(feature = "api")]
pub mod types;

#[cfg(feature = "api")]
pub use contract::{ContractViolation, check_from_empty};
#[cfg(feature = "api")]
pub use engine::{Engine, FileEngine, merged_reconcile};
#[cfg(feature = "api")]
pub use finding::Finding;
#[cfg(feature = "api")]
pub use merge::{
    ConflictEntry, FileItemRequirement, ForbiddenGlobRequirement, ForbiddenGlobRequirements,
    ItemRequirements, KeyedItem, ListRequirements, Resolve, ResolvedForbiddenGlobRequirements,
    ResolvedItemRequirements, ResolvedListRequirements, ResolvedRequirement, compose_item_by,
    compose_optional_field, compose_string_list, compose_string_set, keyed_entries_eq,
    resolve_all_equal, resolve_forbidden_globs, resolve_items, resolve_list, resolve_map,
    resolve_maybe, resolve_scalar, strongest_version_floor,
};
#[cfg(feature = "api")]
pub use requirement::EngineRequirement;
#[cfg(feature = "api")]
pub use toml_helpers::{parse_or_report, parse_version_tuple};
#[cfg(feature = "api")]
pub use types::{ConfigScalar, EngineOutput, OnEmpty, OnEmptyClass, Provenance, Severity};
