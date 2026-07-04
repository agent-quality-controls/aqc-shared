//! Rust toolchain requirement merge logic.

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, ListRequirements, Provenance, Resolve, ScalarAssertion,
};

use super::{
    ResolvedRustToolchainScalarSettings, ResolvedRustToolchainTomlRequirements,
    RustToolchainListSetting, RustToolchainScalarSetting, RustToolchainScalarSettings,
    RustToolchainTomlRequirements,
};

type RustToolchainRequirementInput = Vec<(Provenance, RustToolchainTomlRequirements)>;
type RustToolchainMergeOutput = (ResolvedRustToolchainTomlRequirements, Vec<ConflictEntry>);
type RustToolchainListRequirementsByKey =
    BTreeMap<RustToolchainListSetting, Vec<(Provenance, ListRequirements)>>;
type RustToolchainScalarRequirementInput = Vec<(Provenance, RustToolchainScalarSettings)>;
type RustToolchainScalarAssertionsByKey =
    BTreeMap<RustToolchainScalarSetting, Vec<(Provenance, ScalarAssertion<ConfigScalar>)>>;

impl RustToolchainTomlRequirements {
    #[must_use]
    pub fn merge(reqs: RustToolchainRequirementInput) -> RustToolchainMergeOutput {
        let mut conflicts = Vec::new();
        let scalar_settings = resolve_scalar_settings(
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.scalar_settings.clone()))
                .collect(),
            &mut conflicts,
        );

        let mut lists_by_key: RustToolchainListRequirementsByKey = BTreeMap::new();
        let mut closed_settings = Vec::new();
        for (prov, req) in reqs {
            for (key, list) in req.list_settings {
                lists_by_key
                    .entry(key)
                    .or_default()
                    .push((prov.clone(), normalized_list_requirements(list)));
            }
            if let Some(message) = req.closed_settings {
                closed_settings.push((prov, message));
            }
        }

        let mut list_settings = BTreeMap::new();
        for (key, lists) in lists_by_key {
            let _ = list_settings.insert(
                key,
                aqc_file_engine_core::resolve_list(key.file_key(), lists, &mut conflicts),
            );
        }

        (
            ResolvedRustToolchainTomlRequirements {
                scalar_settings,
                list_settings,
                closed_settings,
            },
            conflicts,
        )
    }
}

fn resolve_scalar_settings(
    input: RustToolchainScalarRequirementInput,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedRustToolchainScalarSettings {
    let mut by_key: RustToolchainScalarAssertionsByKey = BTreeMap::new();
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
            aqc_file_engine_core::push_conflict(
                key_text,
                "scalar-operation-unsupported",
                &items,
                aqc_file_engine_core::render_scalar_assertion,
                conflicts,
            );
            continue;
        }
        if let Some(resolved) = ScalarAssertion::<ConfigScalar>::resolve(key_text, items, conflicts)
        {
            let _ = out.insert(key, resolved);
        }
    }
    out
}

fn normalized_list_requirements(mut list: ListRequirements) -> ListRequirements {
    if let Some((values, message)) = list.exact {
        let mut sorted = values;
        sorted.sort();
        sorted.dedup();
        list.exact = Some((sorted, message));
    }
    list
}
