//! `Cargo.toml` requirement aggregate and merge phase.

mod conflicts;
mod merge;
mod model;
mod resolve;

pub use model::{CargoTomlRequirements, ResolvedCargoTomlRequirements};
