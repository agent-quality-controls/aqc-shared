//! Text file requirement merge logic.

use aqc_file_engine_core::{
    ConflictEntry, ItemAssertionInput, ItemRequirements, Provenance, RequiredItemResolution,
    Resolve, ResolvedItemRequirements, ResolvedRequirement, ScalarAssertion, resolve_items,
};

use super::{ResolvedTextFileRequirements, TextFileRequirement, TextFileRequirements, TextSnippet};

type TextRequirementInput = Vec<(Provenance, TextFileRequirements)>;
type TextMergeOutput = (ResolvedTextFileRequirements, Vec<ConflictEntry>);

impl TextFileRequirements {
    #[must_use]
    pub fn merge(reqs: TextRequirementInput) -> TextMergeOutput {
        let mut conflicts = Vec::new();
        let files = resolve_items(
            "files",
            reqs.into_iter()
                .map(|(prov, req)| (prov, req.files))
                .collect(),
            &mut conflicts,
        );
        (ResolvedTextFileRequirements { files }, conflicts)
    }
}

pub(crate) fn compose_text_file(
    key: &str,
    items: Vec<ItemAssertionInput<TextFileRequirement>>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<RequiredItemResolution<TextFileRequirement>> {
    let path = items.first().map(|(_, (item, _))| item.path.clone())?;
    let exact_contents = resolve_optional_scalar(
        &format!("{key}.exact_contents"),
        &items,
        |item| item.exact_contents.clone(),
        conflicts,
    );
    let executable = resolve_optional_scalar(
        &format!("{key}.executable"),
        &items,
        |item| item.executable.clone(),
        conflicts,
    );
    let required_snippets = resolve_snippets(
        &format!("{key}.required_snippets"),
        &items,
        |item| item.required_snippets.clone(),
        conflicts,
    );
    let merged = TextFileRequirement {
        path,
        exact_contents: exact_contents
            .as_ref()
            .map(|resolved| resolved.merged.clone()),
        required_snippets: unresolved_snippets(&required_snippets),
        executable: executable.as_ref().map(|resolved| resolved.merged.clone()),
    };
    Some(ResolvedRequirement {
        merged,
        collected: items,
    })
}

fn resolve_optional_scalar<T>(
    key: &str,
    items: &[ItemAssertionInput<TextFileRequirement>],
    get: impl Fn(&TextFileRequirement) -> Option<ScalarAssertion<T>>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedRequirement<ScalarAssertion<T>, ScalarAssertion<T>>>
where
    T: aqc_file_engine_core::ScalarValue,
{
    let assertions = items
        .iter()
        .filter_map(|(prov, (item, _))| get(item).map(|assertion| (prov.clone(), assertion)))
        .collect::<Vec<_>>();
    if assertions.is_empty() {
        None
    } else {
        ScalarAssertion::<T>::resolve(key, assertions, conflicts)
    }
}

fn resolve_snippets(
    key: &str,
    items: &[ItemAssertionInput<TextFileRequirement>],
    get: impl Fn(&TextFileRequirement) -> ItemRequirements<TextSnippet>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedItemRequirements<TextSnippet> {
    resolve_items(
        key,
        items
            .iter()
            .map(|(prov, (item, _))| (prov.clone(), get(item)))
            .collect(),
        conflicts,
    )
}

fn unresolved_snippets(
    resolved: &ResolvedItemRequirements<TextSnippet>,
) -> ItemRequirements<TextSnippet> {
    let required = resolved
        .required
        .values()
        .map(|entry| {
            let msg = entry
                .collected
                .first()
                .map_or_else(String::new, |(_, (_, msg))| msg.clone());
            (entry.merged.clone(), msg)
        })
        .collect();
    let forbidden = resolved
        .forbidden
        .values()
        .map(|entry| {
            let msg = entry
                .collected
                .first()
                .map_or_else(String::new, |(_, msg)| msg.clone());
            (entry.merged.clone(), msg)
        })
        .collect();
    let closed = resolved.closed_by.first().map(|(_, msg)| msg.clone());
    ItemRequirements {
        required,
        forbidden,
        closed,
    }
}
