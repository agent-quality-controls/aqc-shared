//! `FileEngine` for `clippy.toml`. Writes msrv, thresholds, bans, and
//! configuration flags via `toml_edit`.
//!
//! See plan refs in the architecture and vertical-slice plans inside
//! `guardrail3/.plans/g3v2-architecture/`.

#[cfg(feature = "api")]
mod engine;
#[cfg(feature = "api")]
mod reconcile;
#[cfg(feature = "api")]
mod requirement;

#[cfg(feature = "api")]
pub use engine::ClippyTomlEngine;
#[cfg(feature = "api")]
pub use requirement::{
    BanEntry, BansAssertion, BoolAssertion, ClippyTomlRequirement, MsrvAssertion, StringAssertion,
    ThresholdsAssertion,
};
