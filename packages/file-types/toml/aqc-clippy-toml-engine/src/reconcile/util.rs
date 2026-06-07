//! Shared helpers used across reconcile submodules.

#![expect(
    clippy::type_complexity,
    reason = "Collected assertions are plainly Vec<(Provenance, A)> and per-key maps of them; the shapes are declared openly at every signature instead of hidden behind wrapper types or aliases (taxonomy decision 2026-06-07)."
)]
use aqc_file_engine_core::Provenance;

/// Collect the provenances of a collected-assertion list.
pub(crate) fn all_provenances<A>(pairs: &[(Provenance, A)]) -> Vec<Provenance> {
    pairs.iter().map(|(p, _)| p.clone()).collect()
}
