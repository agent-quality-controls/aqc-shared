//! `[features]` table assertions.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Msg, OnEmpty, OnEmptyClass};

use super::macros::{impl_keyed_entries_eq, impl_set_resolve};

/// One feature's `(enable-list, message)` entry payload.
pub type FeatureEntries = BTreeMap<String, FeatureEntry>;

/// One feature entry: the enable-list plus the policy message.
pub type FeatureEntry = (BTreeSet<String>, Msg);

/// What must hold about the `[features]` table.
///
/// Equality (and therefore merge agreement) compares feature names and their
/// enable-lists; the policy-authored messages never participate.
#[derive(Debug, Clone)]
pub enum FeatureSetAssertion {
    /// These features must exist with exactly these enable-lists.
    Contains(FeatureEntries),
    /// None of these feature names may exist.
    Excludes(BTreeMap<String, Msg>),
    /// The table must contain exactly these features.
    IsExactly(FeatureEntries),
}

impl_keyed_entries_eq!(FeatureSetAssertion);
impl_set_resolve!(
    FeatureSetAssertion,
    FeatureEntries,
    |entry: &FeatureEntry| entry.0.clone()
);

impl OnEmptyClass for FeatureSetAssertion {
    fn on_empty(&self) -> OnEmpty {
        // A feature entry is self-contained (name -> list); always writable.
        OnEmpty::Writes
    }
}
