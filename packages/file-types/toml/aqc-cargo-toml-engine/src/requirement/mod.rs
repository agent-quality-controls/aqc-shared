//! Declarative requirement and assertion types accepted by `CargoTomlEngine`.
//!
//! One field per addressable target of `Cargo.toml` (struct-of-fields-per-
//! target); each field's value is an assertion enum wrapped in
//! `MergedAssertion` for provenance. The full surface and the locked
//! assertion vocabulary are specified in
//! `guardrail3/.plans/g3v2-architecture/2026-06-06-174328-cargo-engine-control-surface.md`.

mod cargo_toml;
mod dependencies;
mod features;
mod lints;
pub(crate) mod macros;
mod package;
mod profiles;
mod sections;
mod targets;
mod workspace;

#[expect(
    clippy::module_name_repetitions,
    reason = "`CargoTomlRequirement` is the canonical plan-defined name; the `requirement` module suffix is the abstraction it belongs to."
)]
pub use cargo_toml::CargoTomlRequirement;
pub use dependencies::DependencyEntries;
pub use dependencies::DependencyEntry;
pub use dependencies::DependencyKind;
pub use dependencies::DependencyScope;
pub use dependencies::DependencySetAssertion;
pub use dependencies::DependencySpec;
pub use features::FeatureEntries;
pub use features::FeatureEntry;
pub use features::FeatureSetAssertion;
pub use lints::LintEntries;
pub use lints::LintEntry;
pub use lints::LintLevelsAssertion;
pub use lints::PackageLintsAssertion;
pub use package::PackageFieldAssertion;
pub use profiles::ProfileAssertion;
pub use profiles::ProfileFieldAssertion;
pub use profiles::ProfileFields;
pub use sections::ManifestSection;
pub use sections::SectionPresenceAssertion;
pub use targets::TargetFieldAssertion;
pub use targets::TargetTableAssertion;
pub use workspace::WorkspaceFieldAssertion;
