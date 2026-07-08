//! Text file engine struct and erased engine implementation.

use std::path::{Path, PathBuf};

use aqc_file_engine_core::{Engine, EngineFileState, EngineOutput, EngineRequirement, Provenance};

use crate::reconcile;
use crate::requirement::TextFileRequirements;

#[derive(Debug, Default)]
pub struct TextFileEngine;

impl Engine for TextFileEngine {
    fn id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn target_paths(
        &self,
        target_root: &Path,
        reqs: &[(Provenance, Box<dyn EngineRequirement>)],
    ) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        for (_, req) in reqs {
            if let Some(text) = req.as_any().downcast_ref::<TextFileRequirements>() {
                for (item, _) in &text.files.required {
                    paths.push(target_root.join(item.path.as_path()));
                }
                for (item, _) in &text.files.forbidden {
                    paths.push(target_root.join(item.path.as_path()));
                }
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
        let typed = reqs
            .iter()
            .filter_map(|(prov, req)| {
                req.as_any()
                    .downcast_ref::<TextFileRequirements>()
                    .map(|req| (prov.clone(), req.clone()))
            })
            .collect::<Vec<_>>();
        let (requirements, conflicts) = TextFileRequirements::merge(typed);
        let mut files = reconcile::apply(target_root, current, &requirements);
        let mut findings = Vec::new();
        for entry in conflicts {
            findings.push(aqc_file_engine_core::Finding::ConflictingRequirements {
                subject: "text files".to_owned(),
                key: entry.key,
                contributors: entry
                    .contributors
                    .into_iter()
                    .map(|(prov, value)| (prov.policy, value))
                    .collect(),
                reason: entry.reason,
            });
        }
        if files.is_empty() && !current.is_empty() {
            files = current
                .iter()
                .map(|state| aqc_file_engine_core::EngineFileOutput {
                    path: state.path.clone(),
                    expected_bytes: state.bytes.clone().unwrap_or_default(),
                    expected_executable: state.executable,
                    findings: Vec::new(),
                })
                .collect();
        }
        EngineOutput { files, findings }
    }
}
