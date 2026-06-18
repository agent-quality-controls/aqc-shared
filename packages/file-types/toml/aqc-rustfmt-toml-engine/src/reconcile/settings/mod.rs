//! Reconcile top-level `rustfmt.toml` settings.

mod apply;
mod closed;
mod ignore;
mod list;
mod scalar;
mod toml_io;

pub(crate) use apply::apply;
