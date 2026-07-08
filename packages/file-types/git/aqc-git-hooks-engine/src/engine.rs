//! Git hooks engine implementation.

use std::path::{Path, PathBuf};

use aqc_file_engine_core::{Engine, EngineFileState, EngineOutput, EngineRequirement, Provenance};
use aqc_text_file_engine::TextFileEngine;

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
        let text_reqs = text_reqs(reqs)
            .into_iter()
            .map(|(prov, req)| (prov, Box::new(req) as Box<dyn EngineRequirement>))
            .collect::<Vec<_>>();
        TextFileEngine.reconcile(target_root, current, &text_reqs)
    }
}

fn text_reqs(
    reqs: &[(Provenance, Box<dyn EngineRequirement>)],
) -> Vec<(Provenance, aqc_text_file_engine::TextFileRequirements)> {
    reqs.iter()
        .filter_map(|(prov, req)| {
            req.as_any()
                .downcast_ref::<GitHooksRequirements>()
                .map(|req| (prov.clone(), req.files.clone()))
        })
        .collect()
}
