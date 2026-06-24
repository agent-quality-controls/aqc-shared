//! Rustfmt requirement merge logic.

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, ListRequirements, Provenance, Resolve,
    ResolvedForbiddenGlobRequirements, ResolvedListRequirements, ScalarAssertion,
    resolve_forbidden_globs, resolve_list,
};
use globset::GlobBuilder;

use super::{
    ResolvedRustfmtScalarSettings, ResolvedRustfmtTomlRequirements,
    RustfmtForbiddenIgnoreGlobConflictBlocks, RustfmtIgnorePathGlob, RustfmtListSetting,
    RustfmtScalarRequirements, RustfmtScalarSetting, RustfmtTomlRequirements,
};

/// Raw rustfmt requirement inputs grouped with provenance.
type RustfmtRequirementInput = Vec<(Provenance, RustfmtTomlRequirements)>;

/// Result of merging rustfmt requirements.
type RustfmtMergeOutput = (ResolvedRustfmtTomlRequirements, Vec<ConflictEntry>);

/// List setting requirements grouped by rustfmt list setting.
type RustfmtListRequirementsByKey =
    BTreeMap<RustfmtListSetting, Vec<(Provenance, ListRequirements)>>;
/// Scalar setting requirements grouped by provenance.
type RustfmtScalarRequirementInput = Vec<(Provenance, RustfmtScalarRequirements)>;
/// Scalar assertions grouped by rustfmt setting.
type RustfmtScalarAssertionsByKey =
    BTreeMap<RustfmtScalarSetting, Vec<(Provenance, ScalarAssertion<ConfigScalar>)>>;
/// Borrowed scalar assertions for one rustfmt setting.
type RustfmtScalarAssertionSlice<'a> = &'a [(Provenance, ScalarAssertion<ConfigScalar>)];

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

/// Resolves rustfmt scalar settings after checking setting-specific legality.
fn resolve_scalar_settings(
    input: RustfmtScalarRequirementInput,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedRustfmtScalarSettings {
    let mut by_key: RustfmtScalarAssertionsByKey = BTreeMap::new();
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

/// Records a scalar assertion that rustfmt cannot represent for this setting.
fn push_unsupported_scalar_conflict(
    key: &str,
    items: RustfmtScalarAssertionSlice<'_>,
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
            .collect::<std::collections::BTreeSet<_>>();
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
