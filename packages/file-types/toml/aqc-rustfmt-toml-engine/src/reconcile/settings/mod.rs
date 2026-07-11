//! Reconcile top-level `rustfmt.toml` settings.

mod apply;
mod exact;
mod ignore;
mod list;
mod scalar;

pub(crate) use apply::apply;
