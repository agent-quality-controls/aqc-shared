//! Helpers for Cargo lint table requirements.

use std::collections::BTreeMap;

use aqc_file_engine_core::{ItemRequirements, KeyedItem};

use super::cargo_toml::CargoTomlRequirements;
use super::lints::{LintSetting, PackageLintsAssertion};

/// Requirement shape accepted for one Cargo lint table.
pub type CargoLintTableRequirements = ItemRequirements<KeyedItem<LintSetting>>;

/// Build a `CargoTomlRequirements` value for one lint tool table.
#[must_use]
pub fn cargo_lint_table_requirements(
    tool: &str,
    workspace_lints: Option<CargoLintTableRequirements>,
    package_lints: Option<CargoLintTableRequirements>,
) -> Option<CargoTomlRequirements> {
    let mut cargo = CargoTomlRequirements::default();
    if let Some(table) = workspace_lints.filter(has_lint_requirements) {
        let _ = cargo.workspace_lints.insert(tool.to_owned(), table);
    }
    if let Some(table) = package_lints.filter(has_lint_requirements) {
        let mut tools = BTreeMap::new();
        let _ = tools.insert(tool.to_owned(), table);
        cargo.package_lints = Some(PackageLintsAssertion::Inline(tools));
    }
    if cargo.workspace_lints.is_empty() && cargo.package_lints.is_none() {
        None
    } else {
        Some(cargo)
    }
}

/// Returns true when a lint table contains at least one emitted requirement.
fn has_lint_requirements(table: &CargoLintTableRequirements) -> bool {
    !table.required.is_empty()
        || !table.forbidden.is_empty()
        || table.allowed.is_some()
        || table.exact.is_some()
}
