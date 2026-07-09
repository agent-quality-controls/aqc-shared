//! Shared helpers for deny.toml requirement merge.

use aqc_file_engine_core::{
    ConflictEntry, FileItemRequirement, ItemRequirements, ListRequirements, Provenance,
    ScalarAssertion, push_conflict, resolve_items, resolve_list, resolve_maybe,
};

use super::{DenyFeatureBanSpec, DenyTomlRequirements};

pub(super) fn scalar<T>(
    key: &str,
    reqs: &[(Provenance, DenyTomlRequirements)],
    get: impl Fn(&DenyTomlRequirements) -> Option<ScalarAssertion<T>>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<aqc_file_engine_core::ResolvedRequirement<ScalarAssertion<T>, ScalarAssertion<T>>>
where
    T: aqc_file_engine_core::ScalarValue,
{
    resolve_maybe(
        key,
        reqs.iter()
            .map(|(prov, req)| (prov.clone(), get(req)))
            .collect(),
        conflicts,
    )
}

pub(super) fn list(
    key: &str,
    reqs: &[(Provenance, DenyTomlRequirements)],
    get: impl Fn(&DenyTomlRequirements) -> ListRequirements,
    conflicts: &mut Vec<ConflictEntry>,
) -> aqc_file_engine_core::ResolvedListRequirements {
    resolve_list(
        key,
        reqs.iter()
            .map(|(prov, req)| (prov.clone(), normalize_list(get(req))))
            .collect(),
        conflicts,
    )
}

pub(super) fn item<ItemType>(
    key: &str,
    reqs: &[(Provenance, DenyTomlRequirements)],
    get: impl Fn(&DenyTomlRequirements) -> ItemRequirements<ItemType>,
    conflicts: &mut Vec<ConflictEntry>,
) -> aqc_file_engine_core::ResolvedItemRequirements<ItemType>
where
    ItemType: FileItemRequirement,
    ItemType::Identity: ToString,
{
    resolve_items(
        key,
        reqs.iter()
            .map(|(prov, req)| (prov.clone(), get(req)))
            .collect(),
        conflicts,
    )
}

fn normalize_list(mut list: ListRequirements) -> ListRequirements {
    if let Some((values, message)) = list.exact {
        let mut sorted = values;
        sorted.sort();
        sorted.dedup();
        list.exact = Some((sorted, message));
    }
    list
}

pub(super) fn report_feature_overlaps(
    features: &aqc_file_engine_core::ResolvedItemRequirements<DenyFeatureBanSpec>,
    conflicts: &mut Vec<ConflictEntry>,
) {
    for (package, entry) in &features.required {
        let Some(feature) = entry
            .merged
            .allowed_features()
            .intersection(entry.merged.forbidden_features())
            .next()
        else {
            continue;
        };
        push_conflict(
            format!("bans.features.{package}"),
            "feature-allow-deny-overlap",
            &entry.collected,
            |(_, message)| format!("{}: {message}", feature.as_str()),
            conflicts,
        );
    }
}
