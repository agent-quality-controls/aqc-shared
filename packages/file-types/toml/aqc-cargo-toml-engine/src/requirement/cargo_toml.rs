//! The `Cargo.toml` requirement aggregate: the struct-of-fields-per-target
//! plus its merge phase and the erased `EngineRequirement` impl.

#![expect(
    clippy::disallowed_types,
    reason = "`Any` is used only in the `EngineRequirement::as_any` impl; the broker uses it to downcast `Box<dyn EngineRequirement>` back to this concrete `Req` type at dispatch time."
)]

use core::any::Any;

use aqc_file_engine_core::merge::AssertionMap;
use aqc_file_engine_core::{ConflictEntry, EngineRequirement, MergedAssertion};

use super::dependencies::{DependencyScope, DependencySetAssertion};
use super::features::FeatureSetAssertion;
use super::lints::{LintLevelsAssertion, LintsInheritAssertion};
use super::package::PackageFieldAssertion;
use super::profiles::ProfileAssertion;
use super::sections::{ManifestSection, SectionPresenceAssertion};
use super::targets::{TargetFieldAssertion, TargetTableAssertion};
use super::workspace::WorkspaceFieldAssertion;

/// Declarative requirement for the `Cargo.toml` engine.
///
/// One field per addressable target. Each field's value is a
/// `MergedAssertion<...>` (or map thereof) carrying the per-policy
/// contributions.
#[derive(Debug, Clone, Default)]
pub struct CargoTomlRequirement {
    /// `[lints.<tool>]`, keyed by tool. Lint levels are the linter adapters' domain.
    pub lints: AssertionMap<String, LintLevelsAssertion>,
    /// `[workspace.lints.<tool>]`, keyed by tool.
    pub workspace_lints: AssertionMap<String, LintLevelsAssertion>,
    /// The `[lints] workspace = <bool>` member opt-in.
    pub lints_inherit: Option<MergedAssertion<LintsInheritAssertion>>,
    /// `[package].<field>`, keyed by field name.
    pub package_fields: AssertionMap<String, PackageFieldAssertion>,
    /// `[workspace.package].<field>`, keyed by field name.
    pub workspace_package_fields: AssertionMap<String, PackageFieldAssertion>,
    /// `[workspace].<key>` (resolver, members, exclude, default-members).
    pub workspace_fields: AssertionMap<String, WorkspaceFieldAssertion>,
    /// Table-level existence per manifest section.
    pub section_presence: AssertionMap<ManifestSection, SectionPresenceAssertion>,
    /// Dependency tables, keyed by scope (kind + optional `cfg` target).
    pub dependencies: AssertionMap<DependencyScope, DependencySetAssertion>,
    /// `[workspace.dependencies]`.
    pub workspace_dependencies: Option<MergedAssertion<DependencySetAssertion>>,
    /// `[features]`.
    pub features: Option<MergedAssertion<FeatureSetAssertion>>,
    /// `[profile.<name>]`, keyed by profile name.
    pub profiles: AssertionMap<String, ProfileAssertion>,
    /// `[lib].<field>`, keyed by field name (singleton target table).
    pub lib_fields: AssertionMap<String, TargetFieldAssertion>,
    /// `[[bin]]` entries, keyed by target name.
    pub bin_targets: AssertionMap<String, TargetTableAssertion>,
    /// `[[example]]` entries, keyed by target name.
    pub example_targets: AssertionMap<String, TargetTableAssertion>,
    /// `[[test]]` entries, keyed by target name.
    pub test_targets: AssertionMap<String, TargetTableAssertion>,
    /// `[[bench]]` entries, keyed by target name.
    pub bench_targets: AssertionMap<String, TargetTableAssertion>,
    /// `[patch.<registry>]`, keyed by registry name.
    pub patch: AssertionMap<String, DependencySetAssertion>,
}

impl CargoTomlRequirement {
    /// Merge a slice of requirements (all routed to this engine for one file)
    /// into one resolved requirement plus any per-key conflicts.
    ///
    /// Phase 1 of reconciliation: pure, disk-independent. Per field, union the
    /// contributions, then resolve each key (identical → collapse, set/map →
    /// union keys, disagreement → [`ConflictEntry`]). The engine turns each
    /// entry into a `Finding::PolicyConflict`.
    #[must_use]
    #[expect(
        clippy::type_complexity,
        reason = "(Self, Vec<ConflictEntry>) is the natural two-output shape: the resolved requirement plus the conflicts that dropped keys."
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "One union line + one resolve line per manifest target; splitting would scatter the per-field iteration the drift-control rule wants in one place."
    )]
    pub fn merge(reqs: &[&Self]) -> (Self, Vec<ConflictEntry>) {
        let mut u = Self::default();
        for r in reqs {
            aqc_file_engine_core::union_field(&mut u.lints, r.lints.clone());
            aqc_file_engine_core::union_field(&mut u.workspace_lints, r.workspace_lints.clone());
            u.lints_inherit = aqc_file_engine_core::union_optional(
                u.lints_inherit.take(),
                r.lints_inherit.clone(),
            );
            aqc_file_engine_core::union_field(&mut u.package_fields, r.package_fields.clone());
            aqc_file_engine_core::union_field(
                &mut u.workspace_package_fields,
                r.workspace_package_fields.clone(),
            );
            aqc_file_engine_core::union_field(&mut u.workspace_fields, r.workspace_fields.clone());
            aqc_file_engine_core::union_field(&mut u.section_presence, r.section_presence.clone());
            aqc_file_engine_core::union_field(&mut u.dependencies, r.dependencies.clone());
            u.workspace_dependencies = aqc_file_engine_core::union_optional(
                u.workspace_dependencies.take(),
                r.workspace_dependencies.clone(),
            );
            u.features =
                aqc_file_engine_core::union_optional(u.features.take(), r.features.clone());
            aqc_file_engine_core::union_field(&mut u.profiles, r.profiles.clone());
            aqc_file_engine_core::union_field(&mut u.lib_fields, r.lib_fields.clone());
            aqc_file_engine_core::union_field(&mut u.bin_targets, r.bin_targets.clone());
            aqc_file_engine_core::union_field(&mut u.example_targets, r.example_targets.clone());
            aqc_file_engine_core::union_field(&mut u.test_targets, r.test_targets.clone());
            aqc_file_engine_core::union_field(&mut u.bench_targets, r.bench_targets.clone());
            aqc_file_engine_core::union_field(&mut u.patch, r.patch.clone());
        }
        let mut conflicts = Vec::new();
        let out = Self {
            lints: aqc_file_engine_core::resolve_field(
                u.lints,
                |t| format!("[lints.{t}]"),
                &mut conflicts,
            ),
            workspace_lints: aqc_file_engine_core::resolve_field(
                u.workspace_lints,
                |t| format!("[workspace.lints.{t}]"),
                &mut conflicts,
            ),
            lints_inherit: aqc_file_engine_core::resolve_optional(
                "[lints].workspace",
                u.lints_inherit,
                &mut conflicts,
            ),
            package_fields: aqc_file_engine_core::resolve_field(
                u.package_fields,
                |f| format!("[package].{f}"),
                &mut conflicts,
            ),
            workspace_package_fields: aqc_file_engine_core::resolve_field(
                u.workspace_package_fields,
                |f| format!("[workspace.package].{f}"),
                &mut conflicts,
            ),
            workspace_fields: aqc_file_engine_core::resolve_field(
                u.workspace_fields,
                |f| format!("[workspace].{f}"),
                &mut conflicts,
            ),
            section_presence: aqc_file_engine_core::resolve_field(
                u.section_presence,
                |s: &ManifestSection| s.table_path().to_owned(),
                &mut conflicts,
            ),
            dependencies: aqc_file_engine_core::resolve_field(
                u.dependencies,
                DependencyScope::table_path,
                &mut conflicts,
            ),
            workspace_dependencies: aqc_file_engine_core::resolve_optional(
                "[workspace.dependencies]",
                u.workspace_dependencies,
                &mut conflicts,
            ),
            features: aqc_file_engine_core::resolve_optional(
                "[features]",
                u.features,
                &mut conflicts,
            ),
            profiles: aqc_file_engine_core::resolve_field(
                u.profiles,
                |p| format!("[profile.{p}]"),
                &mut conflicts,
            ),
            lib_fields: aqc_file_engine_core::resolve_field(
                u.lib_fields,
                |f| format!("[lib].{f}"),
                &mut conflicts,
            ),
            bin_targets: aqc_file_engine_core::resolve_field(
                u.bin_targets,
                |n| format!("[[bin]].{n}"),
                &mut conflicts,
            ),
            example_targets: aqc_file_engine_core::resolve_field(
                u.example_targets,
                |n| format!("[[example]].{n}"),
                &mut conflicts,
            ),
            test_targets: aqc_file_engine_core::resolve_field(
                u.test_targets,
                |n| format!("[[test]].{n}"),
                &mut conflicts,
            ),
            bench_targets: aqc_file_engine_core::resolve_field(
                u.bench_targets,
                |n| format!("[[bench]].{n}"),
                &mut conflicts,
            ),
            patch: aqc_file_engine_core::resolve_field(
                u.patch,
                |r| format!("[patch.{r}]"),
                &mut conflicts,
            ),
        };
        (out, conflicts)
    }
}

impl EngineRequirement for CargoTomlRequirement {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
