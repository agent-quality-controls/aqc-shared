//! Reconciliation entry points for `CargoTomlEngine`.

mod dependencies;
mod dispatch;
mod features;
mod lints;
mod package_fields;
mod package_lint_tables;
mod package_lints;
mod patch;
mod profiles;
mod section_presence;
mod target_tables;
mod util;
mod workspace_fields;

pub(crate) use dispatch::apply;
