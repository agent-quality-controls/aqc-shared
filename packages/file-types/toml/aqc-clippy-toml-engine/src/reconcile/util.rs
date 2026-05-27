//! Shared helpers used across reconcile submodules.

use aqc_file_engine_core::{MergedAssertion, Provenance};

/// Collect provenances of all contributions in a `MergedAssertion`.
pub(crate) fn all_provenances<A>(merged: &MergedAssertion<A>) -> Vec<Provenance> {
    merged
        .contributions
        .iter()
        .map(|(p, _)| p.clone())
        .collect()
}
