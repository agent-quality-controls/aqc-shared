#[cfg(feature = "api")]
mod runtime;
#[cfg(feature = "api")]
mod types;

#[cfg(feature = "api")]
pub use aqc_file_engine_core::ResolvedRequirement;
#[cfg(feature = "api")]
pub use aqc_file_engine_core::ScalarAssertion;
#[cfg(feature = "api")]
pub use aqc_file_engine_core::{ConflictEntry, Provenance};
#[cfg(feature = "api")]
pub use runtime::ENGINE_ID;
#[cfg(feature = "api")]
pub use runtime::PackageJsonEngine;
#[cfg(feature = "api")]
pub use types::DevEnginePackageManagerRequirements;
#[cfg(feature = "api")]
pub use types::PackageJsonRequirements;
#[cfg(feature = "api")]
pub use types::PackageManagerOnFail;
#[cfg(feature = "api")]
pub use types::ResolvedDevEnginePackageManagerRequirements;
#[cfg(feature = "api")]
pub use types::ResolvedPackageJsonRequirements;
