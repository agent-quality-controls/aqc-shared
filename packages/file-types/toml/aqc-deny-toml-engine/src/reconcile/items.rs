//! Array item deny.toml reconciliation and TOML item rendering.

use std::collections::BTreeSet;

use aqc_file_engine_core::Finding;
use aqc_toml_engine_core::{
    TomlArrayItem, TomlArrayTableItem, TomlItemError, TomlItemField, reconcile_array_items,
    reconcile_array_table_items,
};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, TableLike, Value};

use crate::requirement;

use super::support::string_array_item;

pub(super) fn apply_items(
    doc: &mut DocumentMut,
    requirement: &requirement::ResolvedDenyTomlRequirements,
    findings: &mut Vec<Finding>,
) {
    reconcile_array_items(
        doc,
        item_field(&["graph"], "targets", "graph.targets"),
        &requirement.graph_targets,
        findings,
    );
    reconcile_array_items(
        doc,
        item_field(&["advisories"], "ignore", "advisories.ignore"),
        &requirement.advisories_ignore,
        findings,
    );
    reconcile_array_items(
        doc,
        item_field(&["licenses"], "exceptions", "licenses.exceptions"),
        &requirement.licenses_exceptions,
        findings,
    );
    reconcile_array_table_items(
        doc,
        item_field(&["licenses"], "clarify", "licenses.clarify"),
        &requirement.licenses_clarify,
        findings,
    );
    reconcile_array_items(
        doc,
        item_field(&["bans"], "allow", "bans.allow"),
        &requirement.bans_allow,
        findings,
    );
    reconcile_array_items(
        doc,
        item_field(&["bans"], "deny", "bans.deny"),
        &requirement.bans_deny,
        findings,
    );
    reconcile_array_table_items(
        doc,
        item_field(&["bans"], "features", "bans.features"),
        &requirement.bans_features,
        findings,
    );
    reconcile_array_items(
        doc,
        item_field(&["bans"], "skip", "bans.skip"),
        &requirement.bans_skip,
        findings,
    );
    reconcile_array_items(
        doc,
        item_field(&["bans"], "skip-tree", "bans.skip-tree"),
        &requirement.bans_skip_tree,
        findings,
    );
    reconcile_array_items(
        doc,
        item_field(&["bans", "build"], "globs", "bans.build.globs"),
        &requirement.bans_build_globs,
        findings,
    );
}

fn item_field<'a>(
    table_path: &'a [&'a str],
    field_key: &'a str,
    display_key: &'a str,
) -> TomlItemField<'a> {
    TomlItemField::new(table_path, field_key, display_key)
}

impl TomlArrayItem for requirement::DenyGraphTargetSpec {
    fn read_value(value: &Value) -> Result<Self, TomlItemError> {
        let target = value
            .as_str()
            .ok_or_else(|| TomlItemError::new("graph target must be a string"))?;
        Self::new(target).map_err(item_error)
    }

    fn write_value(&self) -> Value {
        Value::from(self.as_str())
    }

    fn matches_value(current: &Self, required: &Self) -> bool {
        current == required
    }

    fn render_value(&self) -> String {
        self.as_str().to_owned()
    }
}

impl TomlArrayItem for requirement::DenyAdvisoryIgnoreSpec {
    fn read_value(value: &Value) -> Result<Self, TomlItemError> {
        if let Some(id) = value.as_str() {
            return Self::new(id).map_err(item_error);
        }
        let Some(table) = value.as_inline_table() else {
            return Err(TomlItemError::new(
                "advisory ignore must be a string or inline table",
            ));
        };
        let id = table
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "advisory ignore table requires id".to_owned())?;
        table.get("reason").and_then(Value::as_str).map_or_else(
            || Self::new(id).map_err(item_error),
            |reason| Self::with_reason(id, reason).map_err(item_error),
        )
    }

    fn write_value(&self) -> Value {
        if let Some(reason) = self.reason() {
            let mut table = InlineTable::new();
            let _ = table.insert("id", Value::from(self.as_str()));
            let _ = table.insert("reason", Value::from(reason));
            Value::InlineTable(table)
        } else {
            Value::from(self.as_str())
        }
    }

    fn matches_value(current: &Self, required: &Self) -> bool {
        current.as_str() == required.as_str()
            && required
                .reason()
                .is_none_or(|reason| current.reason() == Some(reason))
    }

    fn render_value(&self) -> String {
        render_reason_item("id", self.as_str(), self.reason())
    }
}

impl TomlArrayItem for requirement::DenyLicenseException {
    fn read_value(value: &Value) -> Result<Self, TomlItemError> {
        let Some(table) = value.as_inline_table() else {
            return Err(TomlItemError::new(
                "license exception must be an inline table",
            ));
        };
        let package = read_package_from_inline(table)?;
        let license = table
            .get("allow")
            .or_else(|| table.get("license"))
            .and_then(Value::as_str)
            .ok_or_else(|| "license exception requires allow".to_owned())?;
        Self::new(package, license).map_err(item_error)
    }

    fn write_value(&self) -> Value {
        let mut table = InlineTable::new();
        let _ = table.insert("crate", Value::from(self.as_str()));
        let _ = table.insert("allow", Value::from(self.license()));
        Value::InlineTable(table)
    }

    fn matches_value(current: &Self, required: &Self) -> bool {
        current == required
    }

    fn is_canonical_value(value: &Value) -> bool {
        !inline_table_has_name_key(value)
    }

    fn render_value(&self) -> String {
        format!("crate={} allow={}", self.as_str(), self.license())
    }
}

impl TomlArrayItem for requirement::DenyPackageReasonSpec {
    fn read_value(value: &Value) -> Result<Self, TomlItemError> {
        if let Some(package) = value.as_str() {
            return Self::new(package).map_err(item_error);
        }
        let Some(table) = value.as_inline_table() else {
            return Err(TomlItemError::new(
                "package entry must be a string or inline table",
            ));
        };
        let package = read_package_from_inline(table)?;
        table.get("reason").and_then(Value::as_str).map_or_else(
            || Self::new(package).map_err(item_error),
            |reason| Self::with_reason(package, reason).map_err(item_error),
        )
    }

    fn write_value(&self) -> Value {
        if let Some(reason) = self.reason() {
            let mut table = InlineTable::new();
            let _ = table.insert("crate", Value::from(self.as_str()));
            let _ = table.insert("reason", Value::from(reason));
            Value::InlineTable(table)
        } else {
            Value::from(self.as_str())
        }
    }

    fn matches_value(current: &Self, required: &Self) -> bool {
        current.as_str() == required.as_str()
            && required
                .reason()
                .is_none_or(|reason| current.reason() == Some(reason))
    }

    fn is_canonical_value(value: &Value) -> bool {
        !inline_table_has_name_key(value)
    }

    fn render_value(&self) -> String {
        render_reason_item("crate", self.as_str(), self.reason())
    }
}

impl TomlArrayItem for requirement::DenyBanSpec {
    fn read_value(value: &Value) -> Result<Self, TomlItemError> {
        if let Some(package) = value.as_str() {
            return Self::new(package).map_err(item_error);
        }
        let Some(table) = value.as_inline_table() else {
            return Err(TomlItemError::new(
                "ban entry must be a string or inline table",
            ));
        };
        Self::new(read_package_from_inline(table)?).map_err(item_error)
    }

    fn write_value(&self) -> Value {
        if self.reason().is_none() && self.wrappers().is_empty() {
            return Value::from(self.as_str());
        }
        let mut table = InlineTable::new();
        let _ = table.insert("crate", Value::from(self.as_str()));
        if let Some(reason) = self.reason() {
            let _ = table.insert("reason", Value::from(reason));
        }
        Value::InlineTable(table)
    }

    fn matches_value(current: &Self, required: &Self) -> bool {
        current.as_str() == required.as_str()
    }

    fn is_canonical_value(value: &Value) -> bool {
        !inline_table_has_name_key(value)
    }

    fn render_value(&self) -> String {
        render_reason_item("crate", self.as_str(), self.reason())
    }
}

impl TomlArrayItem for requirement::DenySkipTreeSpec {
    fn read_value(value: &Value) -> Result<Self, TomlItemError> {
        if let Some(package) = value.as_str() {
            return Self::new(package).map_err(item_error);
        }
        let Some(table) = value.as_inline_table() else {
            return Err(TomlItemError::new(
                "skip-tree entry must be a string or inline table",
            ));
        };
        Self::new(read_package_from_inline(table)?).map_err(item_error)
    }

    fn write_value(&self) -> Value {
        let mut table = InlineTable::new();
        let _ = table.insert("crate", Value::from(self.as_str()));
        if let Some(depth) = self.depth().and_then(|value| i64::try_from(value).ok()) {
            let _ = table.insert("depth", Value::from(depth));
        }
        if let Some(reason) = self.reason() {
            let _ = table.insert("reason", Value::from(reason));
        }
        Value::InlineTable(table)
    }

    fn matches_value(current: &Self, required: &Self) -> bool {
        current.as_str() == required.as_str()
    }

    fn is_canonical_value(value: &Value) -> bool {
        !inline_table_has_name_key(value)
    }

    fn render_value(&self) -> String {
        render_reason_item("crate", self.as_str(), self.reason())
    }
}

impl TomlArrayItem for requirement::DenyBuildGlobSpec {
    fn read_value(value: &Value) -> Result<Self, TomlItemError> {
        if let Some(glob) = value.as_str() {
            return Self::new(glob).map_err(item_error);
        }
        let Some(table) = value.as_inline_table() else {
            return Err(TomlItemError::new(
                "build glob must be a string or inline table",
            ));
        };
        let glob = table
            .get("glob")
            .and_then(Value::as_str)
            .ok_or_else(|| "build glob table requires glob".to_owned())?;
        table.get("reason").and_then(Value::as_str).map_or_else(
            || Self::new(glob).map_err(item_error),
            |reason| Self::with_reason(glob, reason).map_err(item_error),
        )
    }

    fn write_value(&self) -> Value {
        if let Some(reason) = self.reason() {
            let mut table = InlineTable::new();
            let _ = table.insert("glob", Value::from(self.as_str()));
            let _ = table.insert("reason", Value::from(reason));
            Value::InlineTable(table)
        } else {
            Value::from(self.as_str())
        }
    }

    fn matches_value(current: &Self, required: &Self) -> bool {
        current.as_str() == required.as_str()
            && required
                .reason()
                .is_none_or(|reason| current.reason() == Some(reason))
    }

    fn render_value(&self) -> String {
        render_reason_item("glob", self.as_str(), self.reason())
    }
}

impl TomlArrayTableItem for requirement::DenyLicenseClarification {
    fn read_table(table: &dyn TableLike) -> Result<Self, TomlItemError> {
        let package = table
            .get("crate")
            .or_else(|| table.get("name"))
            .and_then(Item::as_str)
            .ok_or_else(|| "license clarification requires crate".to_owned())?;
        let expression = table
            .get("expression")
            .and_then(Item::as_str)
            .ok_or_else(|| "license clarification requires expression".to_owned())?;
        Self::new(package, expression).map_err(item_error)
    }

    fn write_table(&self) -> Table {
        let mut table = Table::new();
        table["crate"] = toml_edit::value(self.as_str());
        if let Some(version) = self.version() {
            table["version"] = toml_edit::value(version);
        }
        table["expression"] = toml_edit::value(self.expression());
        if !self.license_files().is_empty() {
            let mut files = Array::new();
            for file in self.license_files() {
                let mut inner = InlineTable::new();
                let _ = inner.insert("path", Value::from(file.path()));
                let _ = inner.insert("hash", Value::from(file.hash()));
                files.push(Value::InlineTable(inner));
            }
            table["license-files"] = Item::Value(Value::Array(files));
        }
        table
    }

    fn matches_table(current: &Self, required: &Self) -> bool {
        current.as_str() == required.as_str()
            && current.expression() == required.expression()
            && required
                .version()
                .is_none_or(|version| current.version() == Some(version))
    }

    fn render_table(&self) -> String {
        format!("crate={} expression={}", self.as_str(), self.expression())
    }
}

impl TomlArrayTableItem for requirement::DenyFeatureBanSpec {
    fn read_table(table: &dyn TableLike) -> Result<Self, TomlItemError> {
        let package = table
            .get("crate")
            .or_else(|| table.get("name"))
            .and_then(Item::as_str)
            .ok_or_else(|| "feature ban requires crate".to_owned())?;
        let allowed = read_string_set(table.get("allow"))?;
        let denied = read_string_set(table.get("deny"))?;
        Self::new(package, allowed, denied).map_err(item_error)
    }

    fn write_table(&self) -> Table {
        let mut table = Table::new();
        table["crate"] = toml_edit::value(self.as_str());
        table["allow"] = string_array_item(
            &self
                .allowed_features()
                .iter()
                .map(|feature| feature.as_str().to_owned())
                .collect::<Vec<_>>(),
        );
        table["deny"] = string_array_item(
            &self
                .forbidden_features()
                .iter()
                .map(|feature| feature.as_str().to_owned())
                .collect::<Vec<_>>(),
        );
        table
    }

    fn matches_table(current: &Self, required: &Self) -> bool {
        current == required
    }

    fn render_table(&self) -> String {
        format!("crate={}", self.as_str())
    }
}

fn read_package_from_inline(table: &InlineTable) -> Result<&str, TomlItemError> {
    table
        .get("crate")
        .or_else(|| table.get("name"))
        .and_then(Value::as_str)
        .ok_or_else(|| TomlItemError::new("package table requires crate"))
}

fn inline_table_has_name_key(value: &Value) -> bool {
    value
        .as_inline_table()
        .is_some_and(|table| table.contains_key("name"))
}

fn read_string_set(
    item: Option<&Item>,
) -> Result<BTreeSet<requirement::DenyNonEmptyString>, TomlItemError> {
    let Some(item) = item else {
        return Ok(BTreeSet::new());
    };
    let Some(array) = item.as_array() else {
        return Err(TomlItemError::new("feature list must be an array"));
    };
    let mut out = BTreeSet::new();
    for value in array {
        let Some(text) = value.as_str() else {
            return Err(TomlItemError::new("feature list values must be strings"));
        };
        let _ = out.insert(requirement::DenyNonEmptyString::new(text).map_err(item_error)?);
    }
    Ok(out)
}

fn item_error(error: impl ToString) -> TomlItemError {
    TomlItemError::new(error.to_string())
}

fn render_reason_item(key: &str, value: &str, reason: Option<&str>) -> String {
    reason.map_or_else(
        || format!("{key}={value}"),
        |reason| format!("{key}={value} reason={reason}"),
    )
}
