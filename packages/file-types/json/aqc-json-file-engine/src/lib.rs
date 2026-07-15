#[cfg(feature = "api")]
mod runtime;
#[cfg(feature = "api")]
mod types;

#[cfg(feature = "api")]
pub use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, ForbiddenGlobRequirements, ItemRequirements, KeyedItem,
    ListRequirements, Provenance, ScalarAssertion, ScalarValue,
};
#[cfg(feature = "api")]
pub use runtime::{ENGINE_ID, JsonFileEngine};
#[cfg(feature = "api")]
pub use types::{JsonFileRequirements, JsonPath, JsonStringGlob, ResolvedJsonFileRequirements};
