//! TOML scalar conversions for deny.toml scalar requirement values.

use aqc_file_engine_core::ScalarValue;
use toml_edit::Item;

use crate::requirement::{
    DenyAdvisoryScope, DenyConfidenceThreshold, DenyDuration, DenyGitSpec, DenyGraphHighlight,
    DenyLintLevel, DenyNonEmptyString,
};

pub(super) trait DenyTomlScalar: ScalarValue {
    fn parse_item(item: &Item) -> Option<Self>;
    fn write_item(value: &Self) -> Item;
    fn render_value(value: &Self) -> String;
}

impl DenyTomlScalar for bool {
    fn parse_item(item: &Item) -> Option<Self> {
        item.as_bool()
    }

    fn write_item(value: &Self) -> Item {
        toml_edit::value(*value)
    }

    fn render_value(value: &Self) -> String {
        value.to_string()
    }
}

impl DenyTomlScalar for u64 {
    fn parse_item(item: &Item) -> Option<Self> {
        item.as_integer()
            .and_then(|value| Self::try_from(value).ok())
    }

    fn write_item(value: &Self) -> Item {
        i64::try_from(*value).map_or_else(|_| toml_edit::value(value.to_string()), toml_edit::value)
    }

    fn render_value(value: &Self) -> String {
        value.to_string()
    }
}

macro_rules! impl_string_scalar {
    ($type_name:ty) => {
        impl DenyTomlScalar for $type_name {
            fn parse_item(item: &Item) -> Option<Self> {
                item.as_str().and_then(|value| Self::new(value).ok())
            }

            fn write_item(value: &Self) -> Item {
                toml_edit::value(value.as_str())
            }

            fn render_value(value: &Self) -> String {
                value.as_str().to_owned()
            }
        }
    };
}

impl_string_scalar!(DenyNonEmptyString);
impl_string_scalar!(DenyDuration);

impl DenyTomlScalar for DenyConfidenceThreshold {
    fn parse_item(item: &Item) -> Option<Self> {
        item.as_float()
            .map(|value| value.to_string())
            .or_else(|| item.as_integer().map(|value| value.to_string()))
            .and_then(|value| Self::new(value).ok())
    }

    fn write_item(value: &Self) -> Item {
        toml_edit::value(value.as_f64())
    }

    fn render_value(value: &Self) -> String {
        value.as_str().to_owned()
    }
}

macro_rules! impl_enum_scalar {
    ($type_name:ty) => {
        impl DenyTomlScalar for $type_name {
            fn parse_item(item: &Item) -> Option<Self> {
                item.as_str().and_then(|value| Self::parse(value).ok())
            }

            fn write_item(value: &Self) -> Item {
                toml_edit::value(value.as_str())
            }

            fn render_value(value: &Self) -> String {
                value.as_str().to_owned()
            }
        }
    };
}

impl_enum_scalar!(DenyLintLevel);
impl_enum_scalar!(DenyAdvisoryScope);
impl_enum_scalar!(DenyGraphHighlight);
impl_enum_scalar!(DenyGitSpec);
