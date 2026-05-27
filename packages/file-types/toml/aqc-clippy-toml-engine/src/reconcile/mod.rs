//! Reconciliation entry points for `ClippyTomlEngine`.
//!
//! Per-target logic lives in submodules to keep each file small.

mod method_bans;
mod msrv;
mod thresholds;
mod util;

pub(crate) use method_bans::apply_method_bans;
pub(crate) use msrv::apply_msrv;
pub(crate) use thresholds::apply_thresholds;
