//! Declarative requirement and assertion types accepted by `RustfmtTomlEngine`.

#![expect(
    clippy::disallowed_types,
    reason = "`Any` is used only for EngineRequirement downcast dispatch."
)]

use core::any::Any;
use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, EngineRequirement, ForbiddenGlobRequirement,
    ForbiddenGlobRequirements, ListRequirements, Provenance, Resolve,
    ResolvedForbiddenGlobRequirements, ResolvedListRequirements, ResolvedRequirement,
    ScalarAssertion, resolve_forbidden_globs, resolve_list,
};
use globset::GlobBuilder;

mod settings;

pub use settings::{RustfmtListSetting, RustfmtScalarSetting};

/// Resolved scalar settings keyed by rustfmt setting name.
pub type ResolvedRustfmtScalarSettings = BTreeMap<
    RustfmtScalarSetting,
    ResolvedRequirement<ScalarAssertion<ConfigScalar>, ScalarAssertion<ConfigScalar>>,
>;

/// Policy provenance entries that closed the rustfmt setting set.
pub type ResolvedRustfmtClosedSettings = Vec<(Provenance, String)>;

/// Raw rustfmt requirement inputs grouped with provenance.
type RustfmtRequirementInput = Vec<(Provenance, RustfmtTomlRequirements)>;

/// Result of merging rustfmt requirements.
type RustfmtMergeOutput = (ResolvedRustfmtTomlRequirements, Vec<ConflictEntry>);

/// List setting requirements grouped by rustfmt list setting.
type RustfmtListRequirementsByKey =
    BTreeMap<RustfmtListSetting, Vec<(Provenance, ListRequirements)>>;

#[derive(Debug, Clone, Default)]
pub struct RustfmtTomlRequirements {
    pub scalar_settings: BTreeMap<RustfmtScalarSetting, ScalarAssertion<ConfigScalar>>,
    pub list_settings: BTreeMap<RustfmtListSetting, ListRequirements>,
    pub forbidden_ignore_path_globs: ForbiddenGlobRequirements<RustfmtIgnorePathGlob>,
    pub closed_settings: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedRustfmtTomlRequirements {
    pub scalar_settings: ResolvedRustfmtScalarSettings,
    pub list_settings: BTreeMap<RustfmtListSetting, ResolvedListRequirements>,
    pub forbidden_ignore_path_globs: ResolvedForbiddenGlobRequirements<RustfmtIgnorePathGlob>,
    pub ignore_glob_conflicts: RustfmtForbiddenIgnoreGlobConflictBlocks,
    pub closed_settings: ResolvedRustfmtClosedSettings,
}

impl RustfmtTomlRequirements {
    #[must_use]
    pub fn merge(reqs: RustfmtRequirementInput) -> RustfmtMergeOutput {
        let mut conflicts = Vec::new();
        let scalar_settings = resolve_scalar_settings(
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.scalar_settings.clone()))
                .collect(),
            &mut conflicts,
        );
        let forbidden_ignore_path_globs = resolve_forbidden_globs(
            RustfmtListSetting::Ignore.file_key(),
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.forbidden_ignore_path_globs.clone()))
                .collect(),
            &mut conflicts,
        );

        let mut lists_by_key: RustfmtListRequirementsByKey = BTreeMap::new();
        let mut closed_settings = Vec::new();
        for (prov, req) in reqs {
            for (key, list) in req.list_settings {
                lists_by_key
                    .entry(key)
                    .or_default()
                    .push((prov.clone(), list));
            }
            if let Some(message) = req.closed_settings {
                closed_settings.push((prov, message));
            }
        }

        let mut list_settings = BTreeMap::new();
        for (key, lists) in lists_by_key {
            let _ = list_settings.insert(key, resolve_list(key.file_key(), lists, &mut conflicts));
        }
        let ignore_glob_conflicts = list_settings.get(&RustfmtListSetting::Ignore).map_or_else(
            RustfmtForbiddenIgnoreGlobConflictBlocks::default,
            |ignore| {
                push_ignore_glob_conflicts(ignore, &forbidden_ignore_path_globs, &mut conflicts)
            },
        );

        (
            ResolvedRustfmtTomlRequirements {
                scalar_settings,
                list_settings,
                forbidden_ignore_path_globs,
                ignore_glob_conflicts,
                closed_settings,
            },
            conflicts,
        )
    }
}

fn resolve_scalar_settings(
    input: Vec<(
        Provenance,
        BTreeMap<RustfmtScalarSetting, ScalarAssertion<ConfigScalar>>,
    )>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedRustfmtScalarSettings {
    let mut by_key: BTreeMap<
        RustfmtScalarSetting,
        Vec<(Provenance, ScalarAssertion<ConfigScalar>)>,
    > = BTreeMap::new();
    for (prov, map) in input {
        for (key, assertion) in map {
            by_key
                .entry(key)
                .or_default()
                .push((prov.clone(), assertion));
        }
    }

    let mut out = BTreeMap::new();
    for (key, items) in by_key {
        let key_text = key.file_key();
        if !items
            .iter()
            .all(|(_, assertion)| key.scalar_assertion_is_legal(assertion))
        {
            push_unsupported_scalar_conflict(key_text, &items, conflicts);
            continue;
        }
        if let Some(resolved) = ScalarAssertion::<ConfigScalar>::resolve(key_text, items, conflicts)
        {
            let _ = out.insert(key, resolved);
        }
    }
    out
}

fn push_unsupported_scalar_conflict(
    key: &str,
    items: &[(Provenance, ScalarAssertion<ConfigScalar>)],
    conflicts: &mut Vec<ConflictEntry>,
) {
    conflicts.push(ConflictEntry {
        key: key.to_owned(),
        reason: "scalar-operation-unsupported".to_owned(),
        contributors: items
            .iter()
            .map(|(prov, assertion)| (prov.clone(), format!("{assertion:?}")))
            .collect(),
    });
}

impl EngineRequirement for RustfmtTomlRequirements {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Path glob used only for forbidden `ignore` entries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RustfmtIgnorePathGlob {
    pub glob: String,
}

impl ForbiddenGlobRequirement for RustfmtIgnorePathGlob {
    type Identity = String;

    fn merge_identity(&self) -> Self::Identity {
        self.glob.clone()
    }

    fn render(&self) -> String {
        self.glob.clone()
    }
}

/// Required `ignore` values and forbidden `ignore` globs that conflict.
#[derive(Debug, Clone, Default)]
pub struct RustfmtForbiddenIgnoreGlobConflictBlocks {
    /// Required `ignore` values blocked during reconciliation.
    pub required: BTreeSet<String>,
    /// Forbidden `ignore` globs blocked during reconciliation.
    pub path_globs: BTreeSet<String>,
}

/// Records conflicts between required ignore entries and forbidden path globs.
fn push_ignore_glob_conflicts(
    ignore: &ResolvedListRequirements,
    globs: &ResolvedForbiddenGlobRequirements<RustfmtIgnorePathGlob>,
    conflicts: &mut Vec<ConflictEntry>,
) -> RustfmtForbiddenIgnoreGlobConflictBlocks {
    let mut blocks = RustfmtForbiddenIgnoreGlobConflictBlocks::default();
    for (glob_identity, glob) in &globs.globs {
        let Ok(compiled) = GlobBuilder::new(&glob.merged.glob).build() else {
            continue;
        };
        let matcher = compiled.compile_matcher();
        let required = ignore
            .contains
            .keys()
            .chain(ignore.exact.iter().flat_map(|exact| exact.merged.iter()))
            .filter(|path| matcher.is_match(path.as_str()))
            .cloned()
            .collect::<BTreeSet<_>>();
        if required.is_empty() {
            continue;
        }
        for path in required {
            let mut contributors = ignore
                .contains
                .get(&path)
                .into_iter()
                .flat_map(|req| req.collected.iter())
                .map(|(prov, _)| (prov.clone(), "required".to_owned()))
                .collect::<Vec<_>>();
            contributors.extend(
                ignore
                    .exact
                    .iter()
                    .flat_map(|req| req.collected.iter())
                    .filter(|(_, (values, _))| values.iter().any(|value| value == &path))
                    .map(|(prov, _)| (prov.clone(), "required".to_owned())),
            );
            contributors.extend(
                glob.collected
                    .iter()
                    .map(|(prov, _)| (prov.clone(), "forbidden".to_owned())),
            );
            conflicts.push(ConflictEntry {
                key: format!("{}.{}", RustfmtListSetting::Ignore.file_key(), path),
                reason: "ignore-path-glob-forbids-required-path".to_owned(),
                contributors,
            });
            let _ = blocks.required.insert(path);
            let _ = blocks.path_globs.insert(glob_identity.clone());
        }
    }
    blocks
}
