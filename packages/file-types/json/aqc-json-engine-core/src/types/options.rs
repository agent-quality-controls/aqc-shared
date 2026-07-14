/// Syntax switches selected by a concrete JSON-family file engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // reason: parse switches map independent JSON syntax capabilities.
pub struct JsonParseOptions {
    pub allow_comments: bool,
    pub allow_loose_object_property_names: bool,
    pub allow_trailing_commas: bool,
    pub allow_missing_commas: bool,
    pub allow_single_quoted_strings: bool,
    pub allow_hexadecimal_numbers: bool,
    pub allow_unary_plus_numbers: bool,
    pub allow_extended_json_numbers: bool,
    pub allow_extended_string_escapes: bool,
    pub allow_extended_whitespace: bool,
    pub allow_utf8_bom: bool,
}
