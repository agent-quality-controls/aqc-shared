//! `ClippyTomlEngine`: the engine struct and its `FileEngine` + `Engine` impls.

use aqc_file_engine_core::{
    Engine, EngineOutput, EngineRequirement, FileEngine, Provenance, merged_reconcile,
};
use aqc_toml_engine_core::parse_or_report;

use crate::reconcile;
use crate::requirement::{ClippyTomlRequirements, ResolvedClippyTomlRequirements};

/// `clippy.toml` engine.
#[derive(Debug, Default)]
pub struct ClippyTomlEngine;

impl FileEngine<ResolvedClippyTomlRequirements> for ClippyTomlEngine {
    fn reconcile(
        current_bytes: Option<&[u8]>,
        requirement: &ResolvedClippyTomlRequirements,
    ) -> EngineOutput {
        let (mut doc, mut findings) = parse_or_report(current_bytes, "clippy.toml");
        if !findings.is_empty() {
            return EngineOutput {
                expected_bytes: Vec::new(),
                findings,
            };
        }
        reconcile::apply(&mut doc, requirement, &mut findings);
        EngineOutput {
            expected_bytes: doc.to_string().into_bytes(),
            findings,
        }
    }
}

impl Engine for ClippyTomlEngine {
    fn id(&self) -> &'static str {
        crate::ENGINE_ID
    }
    fn reconcile(
        &self,
        current_bytes: Option<&[u8]>,
        reqs: &[(Provenance, Box<dyn EngineRequirement>)],
    ) -> EngineOutput {
        merged_reconcile(
            current_bytes,
            reqs,
            ClippyTomlRequirements::merge,
            <Self as FileEngine<ResolvedClippyTomlRequirements>>::reconcile,
        )
    }
}
