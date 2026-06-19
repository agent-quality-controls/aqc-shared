//! Declarative requirement and assertion types accepted by `ClippyTomlEngine`.

mod bans;
mod merge;
mod model;

pub use bans::{BanEntry, ClippyForbiddenGlobConflictBlocks, ClippyPathGlob};
pub use model::{ClippyTomlRequirements, ResolvedClippyTomlRequirements};
