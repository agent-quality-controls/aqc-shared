//! Deny TOML reconciliation entrypoint.

#![allow(
    clippy::missing_docs_in_private_items,
    clippy::too_many_lines,
    clippy::ref_option,
    clippy::needless_pass_by_value,
    reason = "Deny TOML reconciliation is a file-format field map; private helper docs and artificial splits would duplicate cargo-deny field names without improving the public API."
)]

mod apply;
mod items;
mod lists;
mod scalar_apply;
mod scalar_value;
mod scalars;
mod support;

pub(crate) use apply::apply;
