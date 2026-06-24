//! Private requirement composition helpers.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private helpers support local requirement composition."
    )
)]

use aqc_file_engine_core::{ConflictEntry, Provenance};

/// Borrowed provenance-tagged values used for conflict rendering.
type ProvenancedSlice<'a, T> = &'a [(Provenance, T)];

pub(super) fn push_debug_conflict<T: core::fmt::Debug>(
    key: &str,
    reason: &str,
    items: ProvenancedSlice<'_, T>,
    conflicts: &mut Vec<ConflictEntry>,
) {
    conflicts.push(ConflictEntry {
        key: key.to_owned(),
        reason: reason.to_owned(),
        contributors: items
            .iter()
            .map(|(prov, value)| (prov.clone(), format!("{value:?}")))
            .collect(),
    });
}
