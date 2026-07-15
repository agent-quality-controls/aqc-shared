use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConflictEntry, Provenance, ResolvedForbiddenGlobRequirements, ResolvedListRequirements,
    push_rendered_conflict,
};
use globset::GlobBuilder;

use crate::types::{JsonPath, JsonStringGlob};

type ResolvedGlobInputs = BTreeMap<JsonPath, ResolvedForbiddenGlobRequirements<JsonStringGlob>>;

pub(super) fn report_required_glob_conflicts(
    lists: &BTreeMap<JsonPath, ResolvedListRequirements>,
    globs: &ResolvedGlobInputs,
    conflicts: &mut Vec<ConflictEntry>,
) {
    for (path, list) in lists {
        let Some(path_globs) = globs.get(path) else {
            continue;
        };
        let mut required = BTreeMap::<String, Vec<Provenance>>::new();
        for (item, resolved) in &list.contains {
            extend_unique(
                required.entry(item.clone()).or_default(),
                resolved.attribution(),
            );
        }
        if let Some(exact) = &list.exact {
            for item in &exact.merged {
                extend_unique(
                    required.entry(item.clone()).or_default(),
                    exact.attribution(),
                );
            }
        }
        for (item, required_attribution) in required {
            push_required_glob_conflict(path, &item, &required_attribution, path_globs, conflicts);
        }
    }
}

fn extend_unique(target: &mut Vec<Provenance>, values: Vec<Provenance>) {
    for value in values {
        if !target.contains(&value) {
            target.push(value);
        }
    }
}

fn push_required_glob_conflict(
    path: &JsonPath,
    item: &str,
    required_attribution: &[Provenance],
    forbidden: &ResolvedForbiddenGlobRequirements<JsonStringGlob>,
    conflicts: &mut Vec<ConflictEntry>,
) {
    let mut contributors = required_attribution
        .iter()
        .map(|provenance| (provenance.clone(), format!("required {item}")))
        .collect::<Vec<_>>();
    let mut matched = false;
    for requirement in forbidden.globs.values() {
        let Ok(glob) = GlobBuilder::new(&requirement.merged.glob)
            .literal_separator(true)
            .backslash_escape(true)
            .build()
        else {
            continue;
        };
        if !glob.compile_matcher().is_match(item) {
            continue;
        }
        matched = true;
        contributors.extend(requirement.collected.iter().map(|(provenance, _)| {
            (
                provenance.clone(),
                format!("forbidden glob {}", requirement.merged.glob),
            )
        }));
    }
    if !matched {
        return;
    }
    push_rendered_conflict(
        path.clone().child(item).finding_key(),
        "list-required-and-forbidden-glob",
        contributors,
        conflicts,
    );
}
