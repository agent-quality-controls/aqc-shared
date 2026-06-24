//! Clippy disallowed path requirement types.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private conflict helper only supports requirement merging."
    )
)]

use std::collections::BTreeSet;

use aqc_file_engine_core::{
    ConflictEntry, FileItemRequirement, ForbiddenGlobRequirement, Provenance,
    ResolvedForbiddenGlobRequirements, ResolvedItemRequirements, ResolvedRequirement,
    compose_item_by,
};
use globset::GlobBuilder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisallowedEntry {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClippyPathGlob {
    pub glob: String,
}

impl ForbiddenGlobRequirement for ClippyPathGlob {
    type Identity = String;

    fn merge_identity(&self) -> Self::Identity {
        self.glob.clone()
    }

    fn render(&self) -> String {
        self.glob.clone()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ClippyForbiddenGlobConflictBlocks {
    pub required: BTreeSet<String>,
    pub path_globs: BTreeSet<String>,
}

impl FileItemRequirement for DisallowedEntry {
    type Identity = String;

    fn merge_identity(&self) -> Self::Identity {
        self.path.clone()
    }

    fn compose_item(
        key: &str,
        items: Vec<(Provenance, (Self, String))>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self, (Self, String)>> {
        compose_item_by(key, items, |entry| entry.path.clone(), conflicts)
    }
}

pub(crate) fn push_clippy_path_glob_conflicts(
    key: &str,
    reason: &str,
    merged: &ResolvedItemRequirements<DisallowedEntry>,
    globs: &ResolvedForbiddenGlobRequirements<ClippyPathGlob>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ClippyForbiddenGlobConflictBlocks {
    let mut blocks = ClippyForbiddenGlobConflictBlocks::default();
    for (glob_identity, glob) in &globs.globs {
        let Ok(globset) = GlobBuilder::new(&glob.merged.glob).build() else {
            continue;
        };
        let matcher = globset.compile_matcher();
        for (required_path, requirement) in &merged.required {
            if !matcher.is_match(required_path) {
                continue;
            }
            let mut contributors = requirement
                .collected
                .iter()
                .map(|(prov, _)| (prov.clone(), "required".to_owned()))
                .collect::<Vec<_>>();
            contributors.extend(
                glob.collected
                    .iter()
                    .map(|(prov, _)| (prov.clone(), "forbidden".to_owned())),
            );
            conflicts.push(ConflictEntry {
                key: format!("{key}.{required_path}"),
                reason: reason.to_owned(),
                contributors,
            });
            let _ = blocks.required.insert(required_path.clone());
            let _ = blocks.path_globs.insert(glob_identity.clone());
        }
    }
    blocks
}
