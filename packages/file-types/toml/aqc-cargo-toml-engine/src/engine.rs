//! `CargoTomlEngine`: the engine struct and its `FileEngine` + `Engine` impls.

use std::path::{Path, PathBuf};

use aqc_file_engine_core::{
    Engine, EngineOutput, EngineRequirement, FileEngine, Provenance, merged_reconcile,
};
use aqc_toml_engine_core::parse_or_report;

use crate::reconcile;
use crate::requirement::{CargoTomlRequirements, ResolvedCargoTomlRequirements};

/// `Cargo.toml` engine.
#[derive(Debug, Default)]
pub struct CargoTomlEngine;

impl FileEngine<ResolvedCargoTomlRequirements> for CargoTomlEngine {
    fn reconcile(
        current_bytes: Option<&[u8]>,
        requirement: &ResolvedCargoTomlRequirements,
    ) -> EngineOutput {
        let (mut doc, mut findings) = parse_or_report(current_bytes, "Cargo.toml");
        if !findings.is_empty() {
            return EngineOutput::single(Vec::new(), findings);
        }
        reconcile::apply(&mut doc, requirement, &mut findings);
        EngineOutput::single(doc.to_string().into_bytes(), findings)
    }
}

impl Engine for CargoTomlEngine {
    fn id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn target_paths(
        &self,
        workspace_root: &Path,
        _reqs: &[(Provenance, Box<dyn EngineRequirement>)],
    ) -> Vec<PathBuf> {
        vec![workspace_root.join("Cargo.toml")]
    }

    fn reconcile(
        &self,
        workspace_root: &Path,
        current: &[aqc_file_engine_core::EngineFileState],
        reqs: &[(Provenance, Box<dyn EngineRequirement>)],
    ) -> EngineOutput {
        merged_reconcile(
            current,
            self.target_paths(workspace_root, reqs)
                .into_iter()
                .next()
                .unwrap_or_else(PathBuf::new),
            reqs,
            "Cargo.toml",
            CargoTomlRequirements::merge,
            <Self as FileEngine<ResolvedCargoTomlRequirements>>::reconcile,
        )
    }
}
