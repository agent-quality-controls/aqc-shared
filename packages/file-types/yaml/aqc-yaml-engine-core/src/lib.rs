//! Lossless YAML format mechanics for AQC file engines.

#[cfg(feature = "api")]
mod runtime;
#[cfg(feature = "api")]
mod types;

#[cfg(feature = "api")]
pub use runtime::{YamlScalar, apply_scalar_assertion};
#[cfg(feature = "api")]
pub use runtime::{
    parse_yaml_mapping, remove_rejected_effective_root_keys, report_missing_effective_root_keys,
};
#[cfg(feature = "api")]
pub use types::ParsedYamlMapping;
#[cfg(feature = "api")]
pub use types::YamlFieldError;
#[cfg(feature = "api")]
pub use types::YamlFieldValue;
