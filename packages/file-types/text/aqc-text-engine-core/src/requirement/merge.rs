//! Text byte-stream requirement merge logic.

use aqc_file_engine_core::{
    ConflictEntry, ItemRequirements, Provenance, Resolve, ResolvedItemRequirements,
    ResolvedRequirement, ScalarAssertion, resolve_items,
};

use super::{ResolvedTextFileRequirements, TextFileRequirements, TextSnippet};

/// Provenance-tagged text requirements routed to one text-engine target.
type TextRequirementInput = Vec<(Provenance, TextFileRequirements)>;
/// Resolved text requirements plus merge conflicts found during resolution.
type TextMergeOutput = (ResolvedTextFileRequirements, Vec<ConflictEntry>);

impl TextFileRequirements {
    #[must_use]
    #[allow(
        clippy::needless_pass_by_value,
        reason = "merged_reconcile passes owned typed requirements to every engine merge function."
    )]
    pub fn merge(reqs: TextRequirementInput) -> TextMergeOutput {
        let mut conflicts = Vec::new();
        let exact_contents = resolve_optional_scalar(
            "exact_contents",
            &reqs,
            |req| req.exact_contents.clone(),
            &mut conflicts,
        );
        let required_snippets = resolve_snippets(
            "required_snippets",
            &reqs,
            |req| req.required_snippets.clone(),
            &mut conflicts,
        );
        (
            ResolvedTextFileRequirements {
                exact_contents,
                required_snippets,
            },
            conflicts,
        )
    }
}

/// Resolve optional scalar text assertions while preserving provenance.
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

/// Resolve required text snippets through the shared item merge primitive.
fn resolve_snippets(
    key: &str,
    items: &[(Provenance, TextFileRequirements)],
    get: impl Fn(&TextFileRequirements) -> ItemRequirements<TextSnippet>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedItemRequirements<TextSnippet> {
    resolve_items(
        key,
        items
            .iter()
            .map(|(prov, req)| (prov.clone(), get(req)))
            .collect(),
        conflicts,
    )
}
