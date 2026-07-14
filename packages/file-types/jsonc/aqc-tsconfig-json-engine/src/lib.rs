#[cfg(feature = "api")]
mod runtime;
#[cfg(feature = "api")]
mod types;

#[cfg(feature = "api")]
pub use aqc_file_engine_core::merge::ResolvedMap;
#[cfg(feature = "api")]
pub use aqc_file_engine_core::{ConflictEntry, Provenance, ResolvedRequirement, ScalarAssertion};
#[cfg(feature = "api")]
pub use runtime::{ENGINE_ID, TsconfigJsonEngine};
#[cfg(feature = "api")]
pub use types::{
    ResolvedTsconfigJsonRequirements, TsconfigBooleanCompilerOption, TsconfigJsonRequirements,
};
