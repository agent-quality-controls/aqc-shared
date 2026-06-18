//! Declarative requirement and assertion types accepted by `ClippyTomlEngine`.

mod bans;
mod merge;
mod model;
mod scalar;

pub use bans::{BanEntry, ClippyForbiddenGlobConflictBlocks, ClippyPathGlob};
pub use model::{ClippyTomlRequirements, ResolvedClippyTomlRequirements};
pub use scalar::{BoolAssertion, MsrvAssertion, NumericAssertion, StringAssertion};
