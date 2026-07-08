//! Git hooks requirement model.

use core::any::Any;

use aqc_file_engine_core::EngineRequirement;
use aqc_text_engine_core::{ResolvedTextFileRequirements, TextFileRequirements};

#[derive(Debug, Clone, Default)]
pub struct GitHooksRequirements {
    pub files: TextFileRequirements,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedGitHooksRequirements {
    pub files: ResolvedTextFileRequirements,
}

impl EngineRequirement for GitHooksRequirements {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
