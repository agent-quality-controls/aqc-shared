//! Declarative requirement and assertion types accepted by `RustfmtTomlEngine`.

mod merge;
mod model;
mod settings;

pub use model::{
    ResolvedRustfmtClosedSettings, ResolvedRustfmtScalarSettings, ResolvedRustfmtTomlRequirements,
    RustfmtForbiddenIgnoreGlobConflictBlocks, RustfmtIgnorePathGlob, RustfmtScalarRequirements,
    RustfmtTomlRequirements,
};
pub use settings::{RustfmtListSetting, RustfmtScalarSetting};
