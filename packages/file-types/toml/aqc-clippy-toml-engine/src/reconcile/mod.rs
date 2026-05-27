//! Reconciliation entry points for `ClippyTomlEngine`.

mod bans;
mod bools;
mod dispatch;
mod enums;
mod msrv;
mod thresholds;
mod util;

pub(crate) use dispatch::apply;
