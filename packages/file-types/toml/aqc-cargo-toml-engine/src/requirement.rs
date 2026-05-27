//! Declarative requirement and assertion types accepted by `CargoTomlEngine`.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::MergedAssertion;

/// Declarative requirement for the `Cargo.toml` engine.
///
/// One field per addressable section. Each field's value is a
/// `MergedAssertion<...>` (or map thereof) carrying the per-policy
/// contributions.
#[derive(Debug, Clone, Default)]
pub struct CargoTomlRequirement {
    #[expect(
        clippy::type_complexity,
        reason = "BTreeMap<tool, MergedAssertion<LintLevelsAssertion>> is the natural keyed-by-tool shape."
    )]
    pub lints: BTreeMap<String, MergedAssertion<LintLevelsAssertion>>,
    #[expect(
        clippy::type_complexity,
        reason = "Same shape as `lints`, at [workspace.lints]."
    )]
    pub workspace_lints: BTreeMap<String, MergedAssertion<LintLevelsAssertion>>,
    #[expect(
        clippy::type_complexity,
        reason = "Keyed by field name (e.g. edition, rust-version, license)."
    )]
    pub package_fields: BTreeMap<String, MergedAssertion<PackageFieldAssertion>>,
    #[expect(
        clippy::type_complexity,
        reason = "Same shape as `package_fields`, at [workspace.package]."
    )]
    pub workspace_package_fields: BTreeMap<String, MergedAssertion<PackageFieldAssertion>>,
    #[expect(
        clippy::type_complexity,
        reason = "Keyed by profile name (dev, release, test, bench, custom)."
    )]
    pub profiles: BTreeMap<String, MergedAssertion<ProfileAssertion>>,
    #[expect(
        clippy::type_complexity,
        reason = "Keyed by dependency kind (normal / dev / build)."
    )]
    pub dependencies: BTreeMap<DepKind, MergedAssertion<DependencySetAssertion>>,
    pub features: Option<MergedAssertion<FeatureSetAssertion>>,
}

/// What must hold about a `[lints.<tool>]` table.
#[derive(Debug, Clone)]
pub enum LintLevelsAssertion {
    /// Each (lint name, level) mapping must be present on disk.
    Contains(BTreeMap<String, String>),
    /// None of these lint names may be set on disk.
    Excludes(BTreeSet<String>),
    /// The table must equal exactly this mapping.
    IsExactly(BTreeMap<String, String>),
}

/// What must hold about a single `[package].<field>` (or `[workspace.package].<field>`).
///
/// One enum covers both scalar and list-shaped fields. The engine knows
/// from the field key whether it's scalar (`edition`, `license`,
/// `rust-version`) or list (`authors`, `keywords`, `categories`).
#[derive(Debug, Clone)]
pub enum PackageFieldAssertion {
    Equals(String),
    AtLeast(String),
    OneOf(BTreeSet<String>),
    ListContains(Vec<String>),
    ListIsExactly(Vec<String>),
    Present,
    Absent,
}

/// What must hold about a `[profile.<name>]` table.
#[derive(Debug, Clone)]
pub enum ProfileAssertion {
    Fields(BTreeMap<String, ProfileFieldAssertion>),
}

/// What must hold about a single profile field (`opt-level`, `debug`, ...).
#[derive(Debug, Clone)]
pub enum ProfileFieldAssertion {
    Equals(toml_edit::Value),
    OneOf(Vec<toml_edit::Value>),
    Present,
    Absent,
}

/// What must hold about a `[<kind>-dependencies]` table.
#[derive(Debug, Clone)]
pub enum DependencySetAssertion {
    Contains(BTreeMap<String, DependencySpec>),
    Excludes(BTreeSet<String>),
    IsExactly(BTreeMap<String, DependencySpec>),
}

/// Typed shape of one dependency entry. Open-ended; covers the common cases.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DependencySpec {
    pub version: Option<String>,
    pub features: Vec<String>,
    pub default_features: Option<bool>,
    pub optional: Option<bool>,
}

/// Cargo dependency section. Determines which table the engine writes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DepKind {
    Normal,
    Dev,
    Build,
}

/// What must hold about the `[features]` table.
#[derive(Debug, Clone)]
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, BTreeSet<String>> mirrors `[features]`'s actual shape."
)]
pub enum FeatureSetAssertion {
    Contains(BTreeMap<String, BTreeSet<String>>),
    Excludes(BTreeSet<String>),
    IsExactly(BTreeMap<String, BTreeSet<String>>),
}
