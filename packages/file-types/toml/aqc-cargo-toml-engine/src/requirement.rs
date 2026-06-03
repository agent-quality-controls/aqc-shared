//! Declarative requirement and assertion types accepted by `CargoTomlEngine`.

#![expect(
    clippy::disallowed_types,
    reason = "`Any` is used only in the `EngineRequirement::as_any` impl; the broker uses it to downcast `Box<dyn EngineRequirement>` back to this concrete `Req` type at dispatch time."
)]

use core::any::Any;
use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::merge::Contributions;
use aqc_file_engine_core::{
    ConflictEntry, EngineRequirement, MergedAssertion, Provenance, Resolve, merge_map,
    resolve_field, resolve_optional, resolve_scalar, union_field, union_optional,
};

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
    /// The `[lints] workspace = <bool>` opt-in that makes a package inherit the
    /// `[workspace.lints.*]` tables. Without it the lint config is inert.
    pub lints_inherit: Option<MergedAssertion<LintsInheritAssertion>>,
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
    pub fn merge(reqs: &[&Self]) -> (Self, Vec<ConflictEntry>) {
        let mut u = Self::default();
        for r in reqs {
            union_field(&mut u.lints, r.lints.clone());
            union_field(&mut u.workspace_lints, r.workspace_lints.clone());
            union_field(&mut u.package_fields, r.package_fields.clone());
            union_field(
                &mut u.workspace_package_fields,
                r.workspace_package_fields.clone(),
            );
            union_field(&mut u.profiles, r.profiles.clone());
            union_field(&mut u.dependencies, r.dependencies.clone());
            u.features = union_optional(u.features.take(), r.features.clone());
            u.lints_inherit = union_optional(u.lints_inherit.take(), r.lints_inherit.clone());
        }
        let mut conflicts = Vec::new();
        let out = Self {
            lints: resolve_field(u.lints, |t| format!("[lints.{t}]"), &mut conflicts),
            workspace_lints: resolve_field(
                u.workspace_lints,
                |t| format!("[workspace.lints.{t}]"),
                &mut conflicts,
            ),
            package_fields: resolve_field(
                u.package_fields,
                |f| format!("[package].{f}"),
                &mut conflicts,
            ),
            workspace_package_fields: resolve_field(
                u.workspace_package_fields,
                |f| format!("[workspace.package].{f}"),
                &mut conflicts,
            ),
            profiles: resolve_field(u.profiles, |p| format!("[profile.{p}]"), &mut conflicts),
            dependencies: resolve_field(
                u.dependencies,
                |k: &DepKind| match k {
                    DepKind::Normal => "[dependencies]".to_owned(),
                    DepKind::Dev => "[dev-dependencies]".to_owned(),
                    DepKind::Build => "[build-dependencies]".to_owned(),
                },
                &mut conflicts,
            ),
            features: resolve_optional("[features]", u.features, &mut conflicts),
            lints_inherit: resolve_optional("[lints].workspace", u.lints_inherit, &mut conflicts),
        };
        (out, conflicts)
    }
}

/// What must hold about a `[lints.<tool>]` table.
///
/// Map values are `(level, priority, message)` tuples.
///   * `level` is the lint level (`"deny"`, `"warn"`, `"allow"`, `"forbid"`).
///   * `priority` is `Some(i64)` for inline-table form (used for group lints
///     like `clippy::all` with `priority = -1`); `None` for bare-string form.
///   * `message` is the policy-authored explanation surfaced in the
///     `Finding::Mismatch.message` field when this entry disagrees with disk.
///
/// Format choice is policy intent: bare-string for individual lints,
/// inline-table for group lints that need to be applied before per-lint
/// settings. The engine reads both forms but writes whichever form the
/// policy's `priority` slot implies.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, (level, priority, message)> is the explicit per-entry shape; aliasing the inner tuple obscures the policy intent fields."
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintLevelsAssertion {
    /// Each (lint name, (level, priority, message)) mapping must be present on disk.
    Contains(BTreeMap<String, (String, Option<i64>, String)>),
    /// None of these lint names may be set on disk. Value is the message.
    Excludes(BTreeMap<String, String>),
    /// The table must equal exactly these (name -> (level, priority, message)) entries.
    IsExactly(BTreeMap<String, (String, Option<i64>, String)>),
}

/// What must hold about a single `[package].<field>` (or `[workspace.package].<field>`).
///
/// One enum covers both scalar and list-shaped fields. The engine knows
/// from the field key whether it's scalar (`edition`, `license`,
/// `rust-version`) or list (`authors`, `keywords`, `categories`).
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq)]
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

/// `toml_edit::Value` has no `PartialEq`; compare assertions by rendered form,
/// which is stable for the in-memory values adapters construct.
impl PartialEq for ProfileFieldAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equals(a), Self::Equals(b)) => a.to_string() == b.to_string(),
            (Self::OneOf(a), Self::OneOf(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_string() == y.to_string())
            }
            (Self::Present, Self::Present) | (Self::Absent, Self::Absent) => true,
            _ => false,
        }
    }
}

/// What must hold about a `[<kind>-dependencies]` table.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, BTreeSet<String>> mirrors `[features]`'s actual shape."
)]
pub enum FeatureSetAssertion {
    Contains(BTreeMap<String, BTreeSet<String>>),
    Excludes(BTreeSet<String>),
    IsExactly(BTreeMap<String, BTreeSet<String>>),
}

/// Whether `[lints]` inherits the workspace lint tables (`workspace = <bool>`).
///
/// The opt-in that makes `[workspace.lints.*]` actually apply to a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintsInheritAssertion {
    /// `[lints] workspace = <bool>`.
    Workspace(bool),
}

impl EngineRequirement for CargoTomlRequirement {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// `Resolve` impls: each assertion routes its variants to the core strategies.
// Identical contributions collapse; set/map variants union keys; scalar/exact
// disagreement becomes a `ConflictEntry`. Objective merge (msrv/edition max) is
// deferred, so every scalar disagreement conflicts.
// ---------------------------------------------------------------------------

impl Resolve for LintsInheritAssertion {
    fn resolve(
        key: &str,
        contributions: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<Self> {
        resolve_scalar(key, contributions, |a| format!("{a:?}"), conflicts)
    }
}

impl Resolve for PackageFieldAssertion {
    fn resolve(
        key: &str,
        contributions: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<Self> {
        if contributions
            .iter()
            .all(|(_, a)| matches!(a, Self::ListContains(_)))
        {
            return Some(Self::ListContains(union_list_contains(&contributions)));
        }
        resolve_scalar(key, contributions, |a| format!("{a:?}"), conflicts)
    }
}

impl Resolve for ProfileFieldAssertion {
    fn resolve(
        key: &str,
        contributions: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<Self> {
        resolve_scalar(key, contributions, |a| format!("{a:?}"), conflicts)
    }
}

impl Resolve for ProfileAssertion {
    fn resolve(
        key: &str,
        contributions: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<Self> {
        let maps = contributions
            .into_iter()
            .map(|(p, a)| {
                let Self::Fields(m) = a;
                (p, m)
            })
            .collect();
        let merged = merge_map(key, maps, |a| format!("{a:?}"), conflicts);
        Some(Self::Fields(merged))
    }
}

/// `Resolve` for a `Contains(map) | Excludes(set) | IsExactly(map)` assertion:
/// all-`Contains` unions the maps (per-key disagreement conflicts), all-`Excludes`
/// unions the sets, and anything else (mixed kinds, or all-`IsExactly`) must be
/// identical via the scalar rule. `DependencySetAssertion` and
/// `FeatureSetAssertion` share this body verbatim; only the map value type differs.
macro_rules! impl_set_map_resolve {
    ($t:ty) => {
        impl Resolve for $t {
            fn resolve(
                key: &str,
                contributions: Vec<(Provenance, Self)>,
                conflicts: &mut Vec<ConflictEntry>,
            ) -> Option<Self> {
                if contributions
                    .iter()
                    .all(|(_, a)| matches!(a, Self::Contains(_)))
                {
                    let maps = contributions
                        .into_iter()
                        .filter_map(|(p, a)| match a {
                            Self::Contains(m) => Some((p, m)),
                            Self::Excludes(_) | Self::IsExactly(_) => None,
                        })
                        .collect();
                    return Some(Self::Contains(merge_map(
                        key,
                        maps,
                        |s| format!("{s:?}"),
                        conflicts,
                    )));
                }
                if contributions
                    .iter()
                    .all(|(_, a)| matches!(a, Self::Excludes(_)))
                {
                    let out: BTreeSet<String> = contributions
                        .into_iter()
                        .filter_map(|(_, a)| match a {
                            Self::Excludes(s) => Some(s),
                            Self::Contains(_) | Self::IsExactly(_) => None,
                        })
                        .flatten()
                        .collect();
                    return Some(Self::Excludes(out));
                }
                resolve_scalar(key, contributions, |a| format!("{a:?}"), conflicts)
            }
        }
    };
}

impl_set_map_resolve!(DependencySetAssertion);
impl_set_map_resolve!(FeatureSetAssertion);

impl Resolve for LintLevelsAssertion {
    fn resolve(
        key: &str,
        contributions: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<Self> {
        if contributions
            .iter()
            .all(|(_, a)| matches!(a, Self::Contains(_)))
        {
            return Some(Self::Contains(union_lint_contains(
                key,
                contributions,
                conflicts,
            )));
        }
        if contributions
            .iter()
            .all(|(_, a)| matches!(a, Self::Excludes(_)))
        {
            return Some(Self::Excludes(union_lint_excludes(contributions)));
        }
        resolve_scalar(key, contributions, |a| format!("{a:?}"), conflicts)
    }
}

/// Union `ListContains` element lists across contributions, preserving order
/// and deduplicating. Pure union; never conflicts.
fn union_list_contains(contributions: &Contributions<PackageFieldAssertion>) -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    for (_, a) in contributions {
        let PackageFieldAssertion::ListContains(v) = a else {
            continue;
        };
        for it in v {
            if !items.iter().any(|e| e == it) {
                items.push(it.clone());
            }
        }
    }
    items
}

/// Union `Excludes` lint sets across contributions; first message wins.
fn union_lint_excludes(
    contributions: Contributions<LintLevelsAssertion>,
) -> BTreeMap<String, String> {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    for (_, a) in contributions {
        let LintLevelsAssertion::Excludes(map) = a else {
            continue;
        };
        for (lint, msg) in map {
            let _ = out.entry(lint).or_insert(msg);
        }
    }
    out
}

/// Union `Contains` lint maps across contributions, keyed by lint name.
///
/// Two policies setting the same lint to different `(level, priority)` →
/// one conflict keyed `{tool}.{lint}` (the policy-authored message is not part
/// of the disagreement). Same level+priority with different messages collapse.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, (level, priority, message)> mirrors `LintLevelsAssertion::Contains`'s value shape."
)]
fn union_lint_contains(
    key: &str,
    contributions: Contributions<LintLevelsAssertion>,
    conflicts: &mut Vec<ConflictEntry>,
) -> BTreeMap<String, (String, Option<i64>, String)> {
    let mut by_lint: BTreeMap<String, Vec<(Provenance, (String, Option<i64>, String))>> =
        BTreeMap::new();
    for (prov, a) in contributions {
        let LintLevelsAssertion::Contains(map) = a else {
            continue;
        };
        for (lint, v) in map {
            by_lint.entry(lint).or_default().push((prov.clone(), v));
        }
    }
    let render = |v: &(String, Option<i64>, String)| {
        v.1.map_or_else(
            || v.0.clone(),
            |p| format!("{level} (priority {p})", level = v.0),
        )
    };
    let mut out: BTreeMap<String, (String, Option<i64>, String)> = BTreeMap::new();
    for (lint, entries) in by_lint {
        let mut iter = entries.into_iter();
        let Some((first_prov, first_val)) = iter.next() else {
            continue;
        };
        let mut contributors: Vec<(Provenance, String)> = vec![(first_prov, render(&first_val))];
        let mut disagree = false;
        for (prov, v) in iter {
            if (v.0.as_str(), v.1) != (first_val.0.as_str(), first_val.1) {
                disagree = true;
            }
            contributors.push((prov, render(&v)));
        }
        if disagree {
            conflicts.push(ConflictEntry {
                key: format!("{key}.{lint}"),
                reason: "set-key-disagree".to_owned(),
                contributors,
            });
        } else {
            let _ = out.insert(lint, first_val);
        }
    }
    out
}
