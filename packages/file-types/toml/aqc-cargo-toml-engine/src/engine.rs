//! `CargoTomlEngine`: the engine struct and its `FileEngine` + `Engine` impls.

use std::path::{Path, PathBuf};

use aqc_file_engine_core::{
    Engine, EngineOutput, EngineRequirement, FileEngine, merged_reconcile, parse_or_report,
};

use crate::reconcile;
use crate::requirement::CargoTomlRequirement;

/// `Cargo.toml` engine.
#[derive(Debug, Default)]
pub struct CargoTomlEngine;

impl FileEngine<CargoTomlRequirement> for CargoTomlEngine {
    fn reconcile(current_bytes: Option<&[u8]>, requirement: &CargoTomlRequirement) -> EngineOutput {
        let (mut doc, mut findings) = parse_or_report(current_bytes, "Cargo.toml");
        reconcile::apply(&mut doc, requirement, &mut findings);
        EngineOutput {
            expected_bytes: doc.to_string().into_bytes(),
            findings,
        }
    }
}

impl Engine for CargoTomlEngine {
    fn id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn target_path(&self, workspace_root: &Path) -> PathBuf {
        workspace_root.join("Cargo.toml")
    }

    fn reconcile(
        &self,
        current: Option<&[u8]>,
        reqs: &[Box<dyn EngineRequirement>],
    ) -> EngineOutput {
        merged_reconcile(
            current,
            reqs,
            "Cargo.toml",
            CargoTomlRequirement::merge,
            <Self as FileEngine<CargoTomlRequirement>>::reconcile,
        )
    }
}
