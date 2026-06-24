//! Reconciliation entry points for `ClippyTomlEngine`.

mod bools;
mod disallowed;
mod dispatch;
mod enums;
mod msrv;
mod thresholds;

pub(crate) use dispatch::apply;
