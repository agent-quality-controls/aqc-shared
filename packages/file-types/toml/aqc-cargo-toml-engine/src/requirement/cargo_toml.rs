//! The `Cargo.toml` requirement aggregate: the struct-of-fields-per-target
//! plus its merge phase and the erased `EngineRequirement` impl.

#![expect(
    clippy::disallowed_types,
    reason = "`Any` is used only in the `EngineRequirement::as_any` impl; the broker uses it to downcast `Box<dyn EngineRequirement>` back to this concrete `Req` type at dispatch time."
)]

use core::any::Any;
use std::collections::BTreeMap;

use aqc_file_engine_core::{ConflictEntry, EngineRequirement, Provenance};

use super::dependencies::{DependencyScope, DependencySetAssertion};
use super::features::FeatureSetAssertion;
use super::lints::{LintLevelsAssertion, PackageLintsAssertion};
use super::package::PackageFieldAssertion;
use super::profiles::ProfileAssertion;
use super::sections::{ManifestSection, SectionPresenceAssertion};
use super::targets::{TargetFieldAssertion, TargetTableAssertion};
use super::workspace::WorkspaceFieldAssertion;

/// Declarative requirement for the `Cargo.toml` engine.
///
/// One field per addressable target. Each field's value is the collected
/// per-policy assertions: `Vec<(Provenance, A)>` (or a per-key map thereof).
#[expect(
    clippy::type_complexity,
    reason = "Collected assertions are plainly Vec<(Provenance, A)> and per-key maps of them; the shapes are declared openly instead of hidden behind wrapper types or aliases (taxonomy decision 2026-06-07)."
)]
#[derive(Debug, Clone, Default)]
pub struct CargoTomlRequirement {
    /// The `[lints]` either/or decision: inherit the workspace tables or
    /// carry inline `[lints.<tool>]` tables (one key; cargo forbids both).
    pub package_lints: Option<Vec<(Provenance, PackageLintsAssertion)>>,
    /// `[workspace.lints.<tool>]`, keyed by tool.
    pub workspace_lints: BTreeMap<String, Vec<(Provenance, LintLevelsAssertion)>>,
    /// `[package].<field>`, keyed by field name.
    pub package_fields: BTreeMap<String, Vec<(Provenance, PackageFieldAssertion)>>,
    /// `[workspace.package].<field>`, keyed by field name.
    pub workspace_package_fields: BTreeMap<String, Vec<(Provenance, PackageFieldAssertion)>>,
    /// `[workspace].<key>` (resolver, members, exclude, default-members).
    pub workspace_fields: BTreeMap<String, Vec<(Provenance, WorkspaceFieldAssertion)>>,
    /// Table-level existence per manifest section.
    pub section_presence: BTreeMap<ManifestSection, Vec<(Provenance, SectionPresenceAssertion)>>,
    /// Dependency tables, keyed by scope (kind + optional `cfg` target).
    pub dependencies: BTreeMap<DependencyScope, Vec<(Provenance, DependencySetAssertion)>>,
    /// `[workspace.dependencies]`.
    pub workspace_dependencies: Option<Vec<(Provenance, DependencySetAssertion)>>,
    /// `[features]`.
    pub features: Option<Vec<(Provenance, FeatureSetAssertion)>>,
    /// `[profile.<name>]`, keyed by profile name.
    pub profiles: BTreeMap<String, Vec<(Provenance, ProfileAssertion)>>,
    /// `[lib].<field>`, keyed by field name (singleton target table).
    pub lib_fields: BTreeMap<String, Vec<(Provenance, TargetFieldAssertion)>>,
    /// `[[bin]]` entries, keyed by target name.
    pub bin_targets: BTreeMap<String, Vec<(Provenance, TargetTableAssertion)>>,
    /// `[[example]]` entries, keyed by target name.
    pub example_targets: BTreeMap<String, Vec<(Provenance, TargetTableAssertion)>>,
    /// `[[test]]` entries, keyed by target name.
    pub test_targets: BTreeMap<String, Vec<(Provenance, TargetTableAssertion)>>,
    /// `[[bench]]` entries, keyed by target name.
    pub bench_targets: BTreeMap<String, Vec<(Provenance, TargetTableAssertion)>>,
    /// `[patch.<registry>]`, keyed by registry name.
    pub patch: BTreeMap<String, Vec<(Provenance, DependencySetAssertion)>>,
}

impl CargoTomlRequirement {
    /// Merge a slice of requirements (all routed to this engine for one file)
    /// into one resolved requirement plus any per-key conflicts.
    ///
    /// Phase 1 of reconciliation: pure, disk-independent. Per field, union the
    /// contributions, then resolve each key (identical → collapse, set/map →
    /// union keys, disagreement → [`ConflictEntry`]). The engine turns each
    /// entry into a `Finding::ConflictingRequirements`.
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
            u.package_lints = aqc_file_engine_core::union_optional(
                u.package_lints.take(),
                r.package_lints.clone(),
            );
            aqc_file_engine_core::union_field(&mut u.workspace_lints, r.workspace_lints.clone());
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
            package_lints: aqc_file_engine_core::resolve_optional(
                "[lints]",
                u.package_lints,
                &mut conflicts,
            ),
            workspace_lints: aqc_file_engine_core::resolve_field(
                u.workspace_lints,
                |t| format!("[workspace.lints.{t}]"),
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
