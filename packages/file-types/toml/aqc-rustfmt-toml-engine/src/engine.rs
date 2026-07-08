//! `RustfmtTomlEngine`: engine struct and `FileEngine` + `Engine` impls.

use std::path::{Path, PathBuf};

use aqc_file_engine_core::{
    Engine, EngineOutput, EngineRequirement, FileEngine, Provenance, merged_reconcile,
};
use aqc_toml_engine_core::parse_or_report;

use crate::reconcile;
use crate::requirement::{ResolvedRustfmtTomlRequirements, RustfmtTomlRequirements};

/// `rustfmt.toml` engine.
#[derive(Debug, Default)]
pub struct RustfmtTomlEngine;

impl FileEngine<ResolvedRustfmtTomlRequirements> for RustfmtTomlEngine {
    fn reconcile(
        current_bytes: Option<&[u8]>,
        requirement: &ResolvedRustfmtTomlRequirements,
    ) -> EngineOutput {
        let (mut doc, mut findings) = parse_or_report(current_bytes, "rustfmt.toml");
        if !findings.is_empty() {
            return EngineOutput::single(Vec::new(), findings);
        }
        reconcile::apply(&mut doc, requirement, &mut findings);
        EngineOutput::single(doc.to_string().into_bytes(), findings)
    }
}

impl Engine for RustfmtTomlEngine {
    fn id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn target_paths(
        &self,
        workspace_root: &Path,
        _reqs: &[(Provenance, Box<dyn EngineRequirement>)],
    ) -> Vec<PathBuf> {
        vec![workspace_root.join("rustfmt.toml")]
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
            "rustfmt.toml",
            RustfmtTomlRequirements::merge,
            <Self as FileEngine<ResolvedRustfmtTomlRequirements>>::reconcile,
        )
    }
}
