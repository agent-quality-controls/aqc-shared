//! `CargoTomlEngine`: the engine struct and its `FileEngine` + `Engine` impls.

use std::path::{Path, PathBuf};

use aqc_file_engine_core::{
    Engine, EngineOutput, EngineRequirement, FileEngine, Finding, parse_or_report,
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
        let typed: Vec<&CargoTomlRequirement> = reqs
            .iter()
            .filter_map(|r| r.as_any().downcast_ref::<CargoTomlRequirement>())
            .collect();
        match typed.as_slice() {
            [] => EngineOutput {
                expected_bytes: current.map(<[u8]>::to_vec).unwrap_or_default(),
                findings: Vec::new(),
            },
            [one] => <Self as FileEngine<CargoTomlRequirement>>::reconcile(current, one),
            _ => EngineOutput {
                expected_bytes: current.map(<[u8]>::to_vec).unwrap_or_default(),
                findings: vec![Finding::InternalError {
                    message: "multiple requirements routed to one engine; multi-adapter merge is not implemented (v1 routes a single adapter per engine)".to_owned(),
                }],
            },
        }
    }
}
