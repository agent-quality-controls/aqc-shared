//! Git hooks requirement merge.

use aqc_file_engine_core::{ConflictEntry, Provenance};
use aqc_text_engine_core::TextFileRequirements;

use super::{GitHooksRequirements, ResolvedGitHooksRequirements};

type GitHooksRequirementInput = Vec<(Provenance, GitHooksRequirements)>;
type GitHooksMergeOutput = (ResolvedGitHooksRequirements, Vec<ConflictEntry>);

impl GitHooksRequirements {
    #[must_use]
    pub fn merge(reqs: GitHooksRequirementInput) -> GitHooksMergeOutput {
        let text_reqs = reqs
            .into_iter()
            .map(|(prov, req)| (prov, req.files))
            .collect::<Vec<_>>();
        let (files, conflicts) = TextFileRequirements::merge(text_reqs);
        (ResolvedGitHooksRequirements { files }, conflicts)
    }
}
