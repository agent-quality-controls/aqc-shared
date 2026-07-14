//! Typed scalar assertion reconciliation.

use aqc_file_engine_core::ScalarValue;
use aqc_yaml_engine_core::{ParsedYamlMapping, YamlFieldValue, YamlScalar};

use crate::types::{PnpmOnFail, PnpmReleaseAgeMinutes, PnpmTrustPolicy};

impl YamlScalar for PnpmReleaseAgeMinutes {
    fn decode_yaml(value: &YamlFieldValue) -> Option<Self> {
        match value {
            YamlFieldValue::Integer(value) => Self::new(*value).ok(),
            YamlFieldValue::String(_)
            | YamlFieldValue::Boolean(_)
            | YamlFieldValue::StringSequence(_)
            | YamlFieldValue::StringBooleanMapping(_) => None,
        }
    }
    fn write_yaml(document: &ParsedYamlMapping, key: &str, value: &Self) {
        document.set_integer(key, value.get());
    }
}

macro_rules! string_scalar {
    ($type:ty, {$($text:literal => $variant:path),+ $(,)?}) => {
        impl YamlScalar for $type {
            fn decode_yaml(value: &YamlFieldValue) -> Option<Self> {
                match value {
                    YamlFieldValue::String(value) => match value.as_str() {
                        $($text => Some($variant),)+
                        _ => None,
                    },
                    YamlFieldValue::Boolean(_)
                    | YamlFieldValue::Integer(_)
                    | YamlFieldValue::StringSequence(_)
                    | YamlFieldValue::StringBooleanMapping(_) => None,
                }
            }
            fn write_yaml(document: &ParsedYamlMapping, key: &str, value: &Self) {
                document.set_string(key, &value.render());
            }
        }
    };
}

string_scalar!(PnpmOnFail, {
    "download" => PnpmOnFail::Download,
    "error" => PnpmOnFail::Error,
    "warn" => PnpmOnFail::Warn,
    "ignore" => PnpmOnFail::Ignore,
});
string_scalar!(PnpmTrustPolicy, {
    "no-downgrade" => PnpmTrustPolicy::NoDowngrade,
    "off" => PnpmTrustPolicy::Off,
});
