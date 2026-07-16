//! YAML parsing, merged lookup, decoding, and direct mutation.

mod decode;
mod parse;
mod root_keys;
mod scalar;
mod write;

pub(crate) use decode::all_aliases_resolve;
pub(crate) use decode::effective_root_keys;
pub(crate) use decode::read_field;
pub use parse::parse_yaml_mapping;
pub(crate) use parse::root_keys;
pub use root_keys::{remove_rejected_effective_root_keys, report_missing_effective_root_keys};
pub use scalar::YamlScalar;
pub use scalar::apply_scalar_assertion;
pub(crate) use write::remove;
pub(crate) use write::set_boolean;
pub(crate) use write::set_integer;
pub(crate) use write::set_string;
pub(crate) use write::set_string_boolean_mapping;
pub(crate) use write::set_string_sequence;
