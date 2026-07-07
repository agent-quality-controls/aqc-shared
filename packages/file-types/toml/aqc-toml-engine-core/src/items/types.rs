//! Public TOML item reconciliation types.

use aqc_file_engine_core::FileItemRequirement;
use toml_edit::{Table, TableLike, Value};

/// Error returned when a TOML item cannot be parsed as an engine item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TomlItemError {
    message: String,
}

impl TomlItemError {
    /// Create an item parse error.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Return the user-facing error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<String> for TomlItemError {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl From<&str> for TomlItemError {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

/// TOML array-backed item behavior supplied by concrete engines.
pub trait TomlArrayItem: FileItemRequirement {
    /// Parse one TOML array value into an item requirement.
    fn read_value(value: &Value) -> Result<Self, TomlItemError>;
    /// Render the item into one TOML array value.
    fn write_value(&self) -> Value;
    /// Decide whether an existing item satisfies the required item.
    fn matches_value(current: &Self, required: &Self) -> bool;
    /// Return whether an existing TOML value already uses canonical file shape.
    fn is_canonical_value(_value: &Value) -> bool {
        true
    }
    /// Render the item for finding output.
    fn render_value(&self) -> String;
}

/// TOML array-of-tables-backed item behavior supplied by concrete engines.
pub trait TomlArrayTableItem: FileItemRequirement {
    /// Parse one TOML table into an item requirement.
    fn read_table(table: &dyn TableLike) -> Result<Self, TomlItemError>;
    /// Render the item into one TOML table.
    fn write_table(&self) -> Table;
    /// Decide whether an existing item satisfies the required item.
    fn matches_table(current: &Self, required: &Self) -> bool;
    /// Render the item for finding output.
    fn render_table(&self) -> String;
}

/// Location of an item collection in a TOML document.
#[derive(Debug, Clone, Copy)]
pub struct TomlItemField<'a> {
    table_path: &'a [&'a str],
    field_key: &'a str,
    display_key: &'a str,
}

impl<'a> TomlItemField<'a> {
    /// Create an item field location.
    #[must_use]
    pub fn new(table_path: &'a [&'a str], field_key: &'a str, display_key: &'a str) -> Self {
        Self {
            table_path,
            field_key,
            display_key,
        }
    }

    pub(crate) fn table_path(&self) -> &'a [&'a str] {
        self.table_path
    }

    pub(crate) fn field_key(&self) -> &'a str {
        self.field_key
    }

    pub(crate) fn display_key(&self) -> &'a str {
        self.display_key
    }
}
