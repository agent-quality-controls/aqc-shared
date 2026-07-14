use aqc_file_engine_core::{
    Engine, EngineOutput, EngineRequirement, FileEngine, Provenance, merged_reconcile,
};

use crate::runtime::reconcile::reconcile_document;
use crate::{ResolvedTsconfigJsonRequirements, TsconfigJsonRequirements};

pub const ENGINE_ID: &str = "aqc-tsconfig-json-engine";

#[derive(Debug, Default)]
pub struct TsconfigJsonEngine;

impl FileEngine<ResolvedTsconfigJsonRequirements> for TsconfigJsonEngine {
    fn reconcile(
        current_bytes: Option<&[u8]>,
        requirement: &ResolvedTsconfigJsonRequirements,
    ) -> EngineOutput {
        reconcile_document(current_bytes, requirement)
    }
}

impl Engine for TsconfigJsonEngine {
    fn id(&self) -> &'static str {
        ENGINE_ID
    }

    fn reconcile(
        &self,
        current_bytes: Option<&[u8]>,
        requirements: &[(Provenance, Box<dyn EngineRequirement>)],
    ) -> EngineOutput {
        merged_reconcile(
            current_bytes,
            requirements,
            TsconfigJsonRequirements::merge,
            <Self as FileEngine<ResolvedTsconfigJsonRequirements>>::reconcile,
        )
    }
}
