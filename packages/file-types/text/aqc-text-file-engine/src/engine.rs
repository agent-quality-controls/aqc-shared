//! Text byte-stream engine.

use aqc_file_engine_core::{
    Engine, EngineOutput, EngineRequirement, FileEngine, Provenance, merged_reconcile,
};

use crate::reconcile::reconcile_text_file;
use crate::requirement::{ResolvedTextFileRequirements, TextFileRequirements};

#[derive(Debug, Default)]
pub struct TextFileEngine;

impl FileEngine<ResolvedTextFileRequirements> for TextFileEngine {
    fn reconcile(
        current_bytes: Option<&[u8]>,
        resolved_requirements: &ResolvedTextFileRequirements,
    ) -> EngineOutput {
        reconcile_text_file(current_bytes, resolved_requirements)
    }
}

impl Engine for TextFileEngine {
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
            TextFileRequirements::merge,
            <Self as FileEngine<ResolvedTextFileRequirements>>::reconcile,
        )
    }
}
