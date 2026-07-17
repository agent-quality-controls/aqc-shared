//! Forbidden-glob merge functions.

use super::{
    ConflictEntry, ForbiddenGlobRequirement, GlobAssertionGroups, GlobInput, GlobResolutionMap,
    ResolvedForbiddenGlobRequirements, ResolvedRequirement, sort_provenanced,
};

pub fn resolve_forbidden_globs<Glob>(
    _key: &str,
    mut input: Vec<GlobInput<Glob>>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedForbiddenGlobRequirements<Glob>
where
    Glob: ForbiddenGlobRequirement,
    Glob::Identity: ToString,
{
    conflicts.reserve(0);
    sort_provenanced(&mut input);
    let mut by_glob = GlobAssertionGroups::<Glob>::new();

    for (prov, globs) in input {
        for (glob, msg) in globs.globs {
            by_glob
                .entry(glob.merge_identity())
                .or_default()
                .push((prov.clone(), (glob, msg)));
        }
    }

    let mut resolved_globs = GlobResolutionMap::<Glob>::new();
    for (identity, items) in by_glob {
        let Some((_, (first, _))) = items.first() else {
            continue;
        };
        let _ = resolved_globs.insert(
            identity,
            ResolvedRequirement {
                merged: first.clone(),
                collected: items
                    .into_iter()
                    .map(|(prov, (_, msg))| (prov, msg))
                    .collect(),
            },
        );
    }

    ResolvedForbiddenGlobRequirements {
        globs: resolved_globs,
    }
}
