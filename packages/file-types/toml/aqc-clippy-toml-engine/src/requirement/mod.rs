//! Declarative requirement and assertion types accepted by `ClippyTomlEngine`.

mod disallowed;
mod merge;
mod model;

pub use disallowed::{ClippyForbiddenGlobConflictBlocks, ClippyPathGlob, DisallowedEntry};
pub use model::{ClippyTomlRequirements, ResolvedClippyTomlRequirements};
