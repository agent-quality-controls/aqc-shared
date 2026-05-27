//! Declarative requirement and assertion types accepted by `CargoTomlEngine`.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::MergedAssertion;

/// Declarative requirement for the `Cargo.toml` engine.
///
/// One bulk field per target. The `lints` field is keyed by lint tool
/// ("clippy", "rust", "rustdoc"), with each value carrying the per-policy
/// contributions for that tool's `[lints.<tool>]` table.
#[derive(Debug, Clone, Default)]
pub struct CargoTomlRequirement {
    #[expect(
        clippy::type_complexity,
        reason = "BTreeMap<tool, MergedAssertion<LintLevelsAssertion>> is the natural keyed-by-tool shape."
    )]
    pub lints: BTreeMap<String, MergedAssertion<LintLevelsAssertion>>,
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
