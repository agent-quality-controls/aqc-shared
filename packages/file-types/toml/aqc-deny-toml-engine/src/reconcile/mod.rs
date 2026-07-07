//! Deny TOML reconciliation entrypoint.

mod apply;
mod closed;
mod items;
mod lists;
mod scalar_apply;
mod scalar_value;
mod scalars;
mod support;

pub(crate) use apply::apply;
