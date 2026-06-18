//! Reconcile resolved Rustfmt requirements into a TOML document.

mod apply;
pub(crate) mod settings;

pub(crate) use apply::apply;
