//! Shared TOML IO and application helpers for AQC TOML engines.

#[cfg(feature = "api")]
mod finding;
#[cfg(feature = "api")]
mod items;
#[cfg(feature = "api")]
mod lists;
#[cfg(feature = "api")]
mod scalars;
#[cfg(feature = "api")]
mod tables;

#[cfg(feature = "api")]
pub use finding::{attribution, push_mismatch};
#[cfg(feature = "api")]
pub use items::{
    TomlArrayItem, TomlArrayTableItem, TomlItemError, TomlItemField, reconcile_array_items,
    reconcile_array_table_items,
};
#[cfg(feature = "api")]
pub use lists::{
    ListFieldKeyStyle, list_contains, list_item, list_message, list_values, reconcile_list_field,
    reconcile_table_list_field, render_list, report_list_shape, report_list_shape_with_message,
    table_list_values, table_list_values_optional, write_list, write_table_list,
};
#[cfg(feature = "api")]
pub use scalars::{
    ScalarFieldEdit, apply_scalar_assertion, parse_or_report, render_item, render_scalar,
    scalar_assertion_fails, scalar_field_edit, scalar_item, scalar_matches,
};
#[cfg(feature = "api")]
pub use tables::{
    ensure_array_of_tables, ensure_nested, ensure_table, ensure_table_at, table_at, table_at_mut,
    table_ref,
};
