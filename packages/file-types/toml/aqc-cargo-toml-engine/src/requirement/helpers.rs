//! Private requirement composition helpers.

#![expect(
    clippy::type_complexity,
    reason = "Private helpers operate on collected provenance tuple shapes."
)]
#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private helpers support local requirement composition."
    )
)]

use std::collections::BTreeSet;

use aqc_file_engine_core::{ConflictEntry, Provenance};

type StringSetMessages = Vec<(BTreeSet<String>, String)>;
type StringSetMessageOption = Option<(BTreeSet<String>, String)>;

pub(super) fn push_debug_conflict<T: core::fmt::Debug>(
    key: &str,
    reason: &str,
    items: &[(Provenance, T)],
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

pub(super) fn intersect_string_sets_with_message(
    oneofs: StringSetMessages,
) -> StringSetMessageOption {
    let mut iter = oneofs.into_iter();
    let (mut out, msg) = iter.next()?;
    iter.for_each(|(next, _)| {
        out.retain(|item| next.contains(item));
    });
    Some((out, msg))
}
