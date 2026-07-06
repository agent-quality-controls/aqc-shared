//! Reconcile resolved Rust toolchain requirements into a TOML document.

mod apply;
pub(crate) mod settings;
mod settings_support;

pub(crate) use apply::apply;
