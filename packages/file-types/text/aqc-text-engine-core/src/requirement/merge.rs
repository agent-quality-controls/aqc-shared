//! Text byte-stream requirement merge logic.

use aqc_file_engine_core::{
    ConflictEntry, Provenance, Resolve, ResolvedRequirement, ScalarAssertion, resolve_items,
};

use super::{ResolvedTextFileRequirements, TextFileRequirements};

impl TextFileRequirements {
    #[must_use]
    #[allow(
        clippy::needless_pass_by_value,
        reason = "merged_reconcile passes owned typed requirements to every engine merge function."
    )]
    pub fn merge(
        reqs: Vec<(Provenance, Self)>,
    ) -> (ResolvedTextFileRequirements, Vec<ConflictEntry>) {
        let mut conflicts = Vec::new();
        let exact_contents = resolve_optional_scalar(
            "exact_contents",
            &reqs,
            |req| req.exact_contents.clone(),
            &mut conflicts,
        );
        let contents = resolve_items(
            "contents",
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.contents.clone()))
                .collect(),
            &mut conflicts,
        );
        (
            ResolvedTextFileRequirements {
                exact_contents,
                contents,
            },
            conflicts,
        )
    }
}

/// Resolve one optional scalar field while preserving source provenance.
fn resolve_optional_scalar<T>(
    key: &str,
    items: &[(Provenance, TextFileRequirements)],
    get: impl Fn(&TextFileRequirements) -> Option<ScalarAssertion<T>>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedRequirement<ScalarAssertion<T>, ScalarAssertion<T>>>
where
    T: aqc_file_engine_core::ScalarValue,
{
    let assertions = items
        .iter()
        .filter_map(|(prov, req)| get(req).map(|assertion| (prov.clone(), assertion)))
        .collect::<Vec<_>>();
    if assertions.is_empty() {
        None
    } else {
        ScalarAssertion::<T>::resolve(key, assertions, conflicts)
    }
}
