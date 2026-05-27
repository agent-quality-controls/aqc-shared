//! Reconciliation entry points for `CargoTomlEngine`.

mod dependencies;
mod dispatch;
mod features;
mod lints;
mod package_fields;
mod profiles;
mod util;
mod workspace_lints;
mod workspace_package_fields;

pub(crate) use dispatch::apply;
