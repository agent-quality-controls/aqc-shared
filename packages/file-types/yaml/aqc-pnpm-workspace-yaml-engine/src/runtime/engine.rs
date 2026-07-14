//! Engine trait implementations.

use aqc_file_engine_core::{
    Engine, EngineOutput, EngineRequirement, FileEngine, Provenance, merged_reconcile,
};

use crate::runtime::reconcile;
use crate::types::{PnpmWorkspaceYamlRequirements, ResolvedPnpmWorkspaceYamlRequirements};

#[derive(Debug, Default)]
pub struct PnpmWorkspaceYamlEngine;

impl FileEngine<ResolvedPnpmWorkspaceYamlRequirements> for PnpmWorkspaceYamlEngine {
    fn reconcile(
        current_bytes: Option<&[u8]>,
        requirement: &ResolvedPnpmWorkspaceYamlRequirements,
    ) -> EngineOutput {
        reconcile::reconcile(current_bytes, requirement)
    }
}

impl Engine for PnpmWorkspaceYamlEngine {
    fn id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn reconcile(
        &self,
        current_bytes: Option<&[u8]>,
        requirements: &[(Provenance, Box<dyn EngineRequirement>)],
    ) -> EngineOutput {
        merged_reconcile(
            current_bytes,
            requirements,
            PnpmWorkspaceYamlRequirements::merge,
            <Self as FileEngine<ResolvedPnpmWorkspaceYamlRequirements>>::reconcile,
        )
    }
}
