//! Reconcile dependency-shaped tables.

mod apply;
mod removals;
mod required;
mod spec_io;

pub(crate) use apply::{SetRule, apply, apply_set};
