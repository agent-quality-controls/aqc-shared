use std::collections::BTreeSet;

use aqc_file_engine_core::{Finding, Severity};
use jsonc_parser::ast::Value as AstValue;
use jsonc_parser::common::Ranged as _;
use jsonc_parser::cst::CstRootNode;
use jsonc_parser::{CollectOptions, ParseOptions, parse_to_ast};

use super::bind::{bind_masked_numbers, bind_masked_strings};
use super::extensions::{
    MaskedSourceRange, MaskedStringSource, mask_extensions, numeric_separators_are_valid,
};
use crate::{JsonObject, JsonParseOptions};

#[must_use]
pub fn parse_object_or_report(
    current_bytes: Option<&[u8]>,
    file_label: &str,
    options: JsonParseOptions,
) -> (Option<JsonObject>, Vec<Finding>) {
    let parser_options = parser_options(options);
    let parsed = current_bytes.map_or_else(
        || parse_document("{}\n", &parser_options, options),
        |bytes| {
            std::str::from_utf8(bytes)
                .map_err(|error| error.to_string())
                .and_then(|text| parse_document(text, &parser_options, options))
        },
    );
    match parsed {
        Ok(document) => {
            let number_markers = document
                .masked_numbers
                .iter()
                .map(|(marker, _)| marker.clone())
                .collect::<BTreeSet<_>>();
            let (masked_numbers, masked_scalars) =
                bind_masked_numbers(&document.root, document.masked_numbers);
            let masked_strings =
                bind_masked_strings(&document.root, &document.masked_strings, &number_markers);
            (
                Some(JsonObject {
                    root: document.root,
                    masked_numbers,
                    masked_strings,
                    masked_scalars,
                    utf8_bom: document.utf8_bom,
                }),
                Vec::new(),
            )
        }
        Err(error) => (
            None,
            vec![Finding::ParseError {
                message: format!(
                    "{file_label} is not a valid {} object: {error}",
                    if options.allow_comments
                        || options.allow_loose_object_property_names
                        || options.allow_trailing_commas
                        || options.allow_missing_commas
                        || options.allow_single_quoted_strings
                        || options.allow_hexadecimal_numbers
                        || options.allow_unary_plus_numbers
                        || options.allow_extended_json_numbers
                        || options.allow_extended_string_escapes
                        || options.allow_extended_whitespace
                        || options.allow_utf8_bom
                    {
                        "JSONC"
                    } else {
                        "JSON"
                    }
                ),
                severity: Severity::Error,
            }],
        ),
    }
}

struct ParsedDocument {
    root: CstRootNode,
    masked_numbers: Vec<(String, String)>,
    masked_strings: Vec<MaskedStringSource>,
    utf8_bom: bool,
}

enum ParseFailure {
    Syntax(String),
    Validation(String),
}

impl ParseFailure {
    fn into_message(self) -> String {
        match self {
            Self::Syntax(message) | Self::Validation(message) => message,
        }
    }
}

fn parse_document(
    text: &str,
    parser_options: &ParseOptions,
    options: JsonParseOptions,
) -> Result<ParsedDocument, String> {
    let (text, utf8_bom) = text
        .strip_prefix('\u{feff}')
        .map_or((text, false), |text| (text, true));
    if utf8_bom && !options.allow_utf8_bom {
        return Err("UTF-8 BOM is not allowed".to_owned());
    }
    reject_forbidden_whitespace(text, options)?;
    if is_strict_json(options) {
        validate_strict_json(text)?;
    } else {
        reject_unescaped_control_characters(text, options)?;
    }
    let masked = (options.allow_extended_json_numbers || options.allow_extended_string_escapes)
        .then(|| mask_extensions(text, parser_options, options))
        .transpose()?
        .flatten();
    if let Some(masked) = masked {
        parse_cst(
            &masked.text,
            parser_options,
            Some((text, &masked.source_map)),
        )
        .map(|root| ParsedDocument {
            root,
            masked_numbers: masked
                .numbers
                .into_iter()
                .map(|number| (number.marker, number.original))
                .collect(),
            masked_strings: masked.strings,
            utf8_bom,
        })
        .map_err(ParseFailure::into_message)
    } else {
        parse_cst(text, parser_options, None)
            .map(|root| ParsedDocument {
                root,
                masked_numbers: Vec::new(),
                masked_strings: Vec::new(),
                utf8_bom,
            })
            .map_err(ParseFailure::into_message)
    }
}

const fn is_strict_json(options: JsonParseOptions) -> bool {
    !options.allow_comments
        && !options.allow_loose_object_property_names
        && !options.allow_trailing_commas
        && !options.allow_missing_commas
        && !options.allow_single_quoted_strings
        && !options.allow_hexadecimal_numbers
        && !options.allow_unary_plus_numbers
        && !options.allow_extended_json_numbers
        && !options.allow_extended_string_escapes
        && !options.allow_extended_whitespace
        && !options.allow_utf8_bom
}

fn reject_forbidden_whitespace(text: &str, options: JsonParseOptions) -> Result<(), String> {
    let mut characters = text.char_indices().peekable();
    let mut quote = None;
    let mut escaped = false;
    let mut line_comment = false;
    let mut block_comment = false;
    while let Some((index, character)) = characters.next() {
        let next = characters.peek().map(|(_, candidate)| *candidate);
        if line_comment {
            line_comment = !matches!(character, '\n' | '\r');
        } else if block_comment {
            if character == '*' && next == Some('/') {
                block_comment = false;
                let _ = characters.next();
            }
        } else if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
        } else if character == '"' || (character == '\'' && options.allow_single_quoted_strings) {
            quote = Some(character);
        } else if options.allow_comments && character == '/' && next == Some('/') {
            line_comment = true;
            let _ = characters.next();
        } else if options.allow_comments && character == '/' && next == Some('*') {
            block_comment = true;
            let _ = characters.next();
        } else if character.is_whitespace()
            && !matches!(character, ' ' | '\t' | '\n' | '\r')
            && !options.allow_extended_whitespace
        {
            let (line, column) = line_and_column(text, index);
            return Err(format!(
                "invalid whitespace at line {line} column {}",
                column.saturating_add(1)
            ));
        }
    }
    Ok(())
}

#[allow(clippy::disallowed_methods)] // reason: serde_json provides strict JSON diagnostics.
fn validate_strict_json(text: &str) -> Result<(), String> {
    serde_json::from_str::<Box<serde_json::value::RawValue>>(text)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn reject_unescaped_control_characters(
    text: &str,
    options: JsonParseOptions,
) -> Result<(), String> {
    let bytes = text.as_bytes();
    let mut index = 0;
    let mut quote = None;
    let mut escaped = false;
    let mut line_comment = false;
    let mut block_comment = false;
    while let Some(&byte) = bytes.get(index) {
        let next = bytes.get(index.saturating_add(1)).copied();
        if line_comment {
            line_comment = byte != b'\n' && byte != b'\r';
        } else if block_comment {
            if byte == b'*' && next == Some(b'/') {
                block_comment = false;
                index = index.saturating_add(1);
            }
        } else if let Some(delimiter) = quote {
            let allowed_line_continuation =
                escaped && options.allow_extended_string_escapes && matches!(byte, b'\n' | b'\r');
            if allowed_line_continuation && byte == b'\r' && next == Some(b'\n') {
                index = index.saturating_add(1);
            }
            if byte < b' ' && !allowed_line_continuation {
                let (line, column) = line_and_column(text, index);
                return Err(format!(
                    "unescaped control character in string at line {line} column {}",
                    column.saturating_add(1)
                ));
            }
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == delimiter {
                quote = None;
            }
        } else if byte == b'"' || (byte == b'\'' && options.allow_single_quoted_strings) {
            quote = Some(byte);
        } else if options.allow_comments && byte == b'/' && next == Some(b'/') {
            line_comment = true;
            index = index.saturating_add(1);
        } else if options.allow_comments && byte == b'/' && next == Some(b'*') {
            block_comment = true;
            index = index.saturating_add(1);
        } else if byte == b'_' && numeric_separator_is_invalid(bytes, index) {
            let (line, column) = line_and_column(text, index);
            return Err(format!(
                "invalid numeric separator at line {line} column {}",
                column.saturating_add(1)
            ));
        }
        index = index.saturating_add(1);
    }
    Ok(())
}

fn numeric_separator_is_invalid(bytes: &[u8], index: usize) -> bool {
    let start = bytes
        .get(..index)
        .and_then(|prefix| {
            prefix.iter().rposition(|byte| {
                byte.is_ascii_whitespace() || matches!(*byte, b'{' | b'[' | b',' | b':')
            })
        })
        .map_or(0, |delimiter| delimiter.saturating_add(1));
    let end = bytes
        .get(index..)
        .and_then(|suffix| {
            suffix.iter().position(|byte| {
                byte.is_ascii_whitespace() || matches!(*byte, b'}' | b']' | b',' | b':' | b'/')
            })
        })
        .map_or(bytes.len(), |offset| index.saturating_add(offset));
    let token = bytes.get(start..end).unwrap_or_default();
    let unsigned = token
        .strip_prefix(b"-")
        .or_else(|| token.strip_prefix(b"+"))
        .unwrap_or(token);
    let starts_as_number = unsigned.first().is_some_and(u8::is_ascii_digit)
        || (unsigned.first() == Some(&b'.') && unsigned.get(1).is_some_and(u8::is_ascii_digit));
    starts_as_number
        && std::str::from_utf8(unsigned).is_ok_and(|number| !numeric_separators_are_valid(number))
}

fn parse_cst(
    text: &str,
    options: &ParseOptions,
    original: Option<(&str, &[MaskedSourceRange])>,
) -> Result<CstRootNode, ParseFailure> {
    let parsed = parse_to_ast(text, &CollectOptions::default(), options)
        .map_err(|error| ParseFailure::Syntax(render_parse_error(&error, original)))?;
    validate_ast(parsed.value.as_ref(), text, original).map_err(ParseFailure::Validation)?;
    CstRootNode::parse(text, options)
        .map_err(|error| ParseFailure::Syntax(render_parse_error(&error, original)))
}

fn validate_ast(
    value: Option<&AstValue<'_>>,
    text: &str,
    original: Option<(&str, &[MaskedSourceRange])>,
) -> Result<(), String> {
    let Some(AstValue::Object(object)) = value else {
        return Err("root value must be an object".to_owned());
    };
    reject_object_duplicates(object, text, original)
}

fn reject_ast_duplicates(
    value: &AstValue<'_>,
    text: &str,
    original: Option<(&str, &[MaskedSourceRange])>,
) -> Result<(), String> {
    match value {
        AstValue::Object(object) => reject_object_duplicates(object, text, original)?,
        AstValue::Array(array) => {
            for element in &array.elements {
                reject_ast_duplicates(element, text, original)?;
            }
        }
        AstValue::StringLit(_)
        | AstValue::NumberLit(_)
        | AstValue::BooleanLit(_)
        | AstValue::NullKeyword(_) => {}
    }
    Ok(())
}

fn reject_object_duplicates(
    object: &jsonc_parser::ast::Object<'_>,
    text: &str,
    original: Option<(&str, &[MaskedSourceRange])>,
) -> Result<(), String> {
    let mut names = BTreeSet::new();
    for property in &object.properties {
        let name = property.name.as_str();
        if !names.insert(name) {
            let (diagnostic_text, end) = original
                .map_or_else(
                    || (text, property.name.end()),
                    |(source, source_map)| {
                        (source, original_offset(property.name.end(), source_map))
                    },
                );
            let (line, column) = line_and_column(diagnostic_text, end);
            return Err(format!(
                "duplicate object member `{name}` at line {line} column {column}"
            ));
        }
        reject_ast_duplicates(&property.value, text, original)?;
    }
    Ok(())
}

pub(super) fn line_and_column(text: &str, end: usize) -> (usize, usize) {
    let prefix = text.get(..end).unwrap_or(text);
    let mut line: usize = 1;
    let mut column: usize = 0;
    let mut characters = prefix.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\r' {
            if characters.peek() == Some(&'\n') {
                let _ = characters.next();
            }
            line = line.saturating_add(1);
            column = 0;
        } else if matches!(character, '\n' | '\u{2028}' | '\u{2029}') {
            line = line.saturating_add(1);
            column = 0;
        } else {
            column = column.saturating_add(1);
        }
    }
    (line, column)
}

fn render_parse_error(
    error: &jsonc_parser::errors::ParseError,
    original: Option<(&str, &[MaskedSourceRange])>,
) -> String {
    let Some((source, source_map)) = original else {
        return error.to_string();
    };
    let offset = original_offset(error.range().start, source_map);
    let (line, column) = line_and_column(source, offset);
    format!(
        "{} on line {line} column {}",
        error.kind(),
        column.saturating_add(1)
    )
}

fn original_offset(offset: usize, source_map: &[MaskedSourceRange]) -> usize {
    let mut masked_cursor: usize = 0;
    let mut original_cursor: usize = 0;
    for range in source_map {
        if offset < range.masked_start {
            return original_cursor.saturating_add(offset.saturating_sub(masked_cursor));
        }
        if offset < range.masked_end {
            let relative = offset.saturating_sub(range.masked_start);
            let original_len = range.original_end.saturating_sub(range.original_start);
            return range
                .original_start
                .saturating_add(relative.min(original_len));
        }
        masked_cursor = range.masked_end;
        original_cursor = range.original_end;
    }
    original_cursor.saturating_add(offset.saturating_sub(masked_cursor))
}

const fn parser_options(options: JsonParseOptions) -> ParseOptions {
    ParseOptions {
        allow_comments: options.allow_comments,
        allow_loose_object_property_names: options.allow_loose_object_property_names,
        allow_trailing_commas: options.allow_trailing_commas,
        allow_missing_commas: options.allow_missing_commas,
        allow_single_quoted_strings: options.allow_single_quoted_strings,
        allow_hexadecimal_numbers: options.allow_hexadecimal_numbers,
        allow_unary_plus_numbers: options.allow_unary_plus_numbers,
    }
}
