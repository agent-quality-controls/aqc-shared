//! Reconcile resolved Rust toolchain requirements into a TOML document.

#![allow(
    clippy::missing_docs_in_private_items,
    reason = "Rust toolchain reconciliation helpers mirror file fields and are private to the engine."
)]

mod apply;
pub(crate) mod settings;
mod settings_support;

pub(crate) use apply::apply;
