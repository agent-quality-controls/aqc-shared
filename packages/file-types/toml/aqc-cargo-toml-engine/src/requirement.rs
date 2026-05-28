//! Declarative requirement and assertion types accepted by `CargoTomlEngine`.

#![expect(
    clippy::disallowed_types,
    reason = "`Any` is used only in the `EngineRequirement::as_any` impl; the broker uses it to downcast `Box<dyn EngineRequirement>` back to this concrete `Req` type at dispatch time."
)]

use core::any::Any;
use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{EngineRequirement, MergedAssertion};

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
///
/// Map values are `(level, message)` tuples where `message` is the
/// policy-authored explanation surfaced in the `Finding::Mismatch.message`
/// field when this lint disagrees with disk.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, (level, message)> is the explicit per-entry shape; aliasing the inner tuple obscures the (value, message) pattern used uniformly across assertion types."
)]
#[derive(Debug, Clone)]
pub enum LintLevelsAssertion {
    /// Each (lint name, (level, message)) mapping must be present on disk.
    Contains(BTreeMap<String, (String, String)>),
    /// None of these lint names may be set on disk. Value is the message.
    Excludes(BTreeMap<String, String>),
    /// The table must equal exactly these (name -> (level, message)) entries.
    IsExactly(BTreeMap<String, (String, String)>),
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

impl EngineRequirement for CargoTomlRequirement {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
