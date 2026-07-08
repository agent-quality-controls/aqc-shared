//! Git hooks engine implementation.

use std::path::{Path, PathBuf};

use aqc_file_engine_core::{
    Engine, EngineFileState, EngineOutput, EngineRequirement, Finding, Provenance,
};
use aqc_text_engine_core::{TextFileRequirements, reconcile_text_files};

use crate::requirement::GitHooksRequirements;

#[derive(Debug, Default)]
pub struct GitHooksEngine;

impl Engine for GitHooksEngine {
    fn id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn target_paths(
        &self,
        target_root: &Path,
        reqs: &[(Provenance, Box<dyn EngineRequirement>)],
    ) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        for (_, req) in text_reqs(reqs) {
            for (file, _) in &req.files.required {
                paths.push(target_root.join(file.path.as_path()));
            }
            for (file, _) in &req.files.forbidden {
                paths.push(target_root.join(file.path.as_path()));
            }
        }
        paths.sort();
        paths.dedup();
        paths
    }

    fn reconcile(
        &self,
        target_root: &Path,
        current: &[EngineFileState],
        reqs: &[(Provenance, Box<dyn EngineRequirement>)],
    ) -> EngineOutput {
        let (requirements, conflicts) = TextFileRequirements::merge(text_reqs(reqs));
        let files = reconcile_text_files(target_root, current, &requirements);
        let findings = conflicts
            .into_iter()
            .map(|entry| Finding::ConflictingRequirements {
                subject: "git hooks".to_owned(),
                key: entry.key,
                contributors: entry
                    .contributors
                    .into_iter()
                    .map(|(prov, value)| (prov.policy, value))
                    .collect(),
                reason: entry.reason,
            })
            .collect();
        EngineOutput { files, findings }
    }
}

fn text_reqs(
    reqs: &[(Provenance, Box<dyn EngineRequirement>)],
) -> Vec<(Provenance, TextFileRequirements)> {
    reqs.iter()
        .filter_map(|(prov, req)| {
            req.as_any()
                .downcast_ref::<GitHooksRequirements>()
                .map(|req| (prov.clone(), req.files.clone()))
        })
        .collect()
}
