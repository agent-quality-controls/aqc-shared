use jsonc_parser::{ParseOptions, cst::CstRootNode};
use tree_sitter::{Node, Parser};

use crate::JsonParseOptions;

use super::parse::line_and_column;

pub(super) struct MaskedDocument {
    pub(super) text: String,
    pub(super) numbers: Vec<MaskedNumberSource>,
    pub(super) strings: Vec<MaskedStringSource>,
    pub(super) source_map: Vec<MaskedSourceRange>,
}

pub(super) struct MaskedStringSource {
    pub(super) masked: String,
    pub(super) original: String,
    pub(super) unrepresentable: bool,
}

pub(super) struct MaskedNumberSource {
    pub(super) marker: String,
    pub(super) original: String,
}

#[derive(Clone, Copy)]
pub(super) struct MaskedSourceRange {
    pub(super) original_start: usize,
    pub(super) original_end: usize,
    pub(super) masked_start: usize,
    pub(super) masked_end: usize,
}

pub(super) fn mask_extensions(
    text: &str,
    parser_options: &ParseOptions,
    options: JsonParseOptions,
) -> Result<Option<MaskedDocument>, String> {
    let replacements = extension_replacements(text, parser_options, options)?;
    if replacements.is_empty() {
        return Ok(None);
    }
    build_masked_document(text, replacements).map(Some)
}

fn extension_replacements(
    text: &str,
    parser_options: &ParseOptions,
    options: JsonParseOptions,
) -> Result<Vec<Replacement>, String> {
    let wrapped = format!("({text})");
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_javascript::LANGUAGE.into())
        .map_err(|error| error.to_string())?;
    let tree = parser
        .parse(&wrapped, None)
        .ok_or_else(|| "JavaScript number parser returned no syntax tree".to_owned())?;
    let mut ranges = Vec::new();
    if options.allow_extended_json_numbers {
        collect_unsupported_numbers(tree.root_node(), &wrapped, parser_options, &mut ranges)?;
    }
    let mut replacements = ranges
        .into_iter()
        .map(|(start, end)| Replacement::number(start, end))
        .collect::<Vec<_>>();
    if options.allow_extended_string_escapes {
        collect_string_replacements(tree.root_node(), &wrapped, &mut replacements)?;
    }
    replacements.sort_unstable_by_key(|replacement| replacement.original_start);
    replacements.dedup_by_key(|replacement| (replacement.original_start, replacement.original_end));
    Ok(replacements)
}

fn build_masked_document(
    text: &str,
    replacements: Vec<Replacement>,
) -> Result<MaskedDocument, String> {
    let mut masked = String::with_capacity(text.len());
    let mut masked_numbers = Vec::new();
    let mut masked_strings = Vec::new();
    let mut source_map = Vec::new();
    let surrogate_marker = "_".repeat(text.len().saturating_add(1));
    let mut cursor = 0;
    for (index, replacement) in replacements.into_iter().enumerate() {
        let (start, end) = replacement.source_range()?;
        let original = text
            .get(start..end)
            .ok_or_else(|| "number range is not valid UTF-8".to_owned())?
            .to_owned();
        let prefix = text
            .get(cursor..start)
            .ok_or_else(|| "number prefix range is not valid UTF-8".to_owned())?;
        masked.push_str(prefix);
        let masked_start = masked.len();
        let normalized_string = matches!(replacement.kind, ReplacementKind::String)
            .then(|| normalize_extended_string(&original, &surrogate_marker))
            .transpose()
            .map_err(|failure| {
                let (line, column) = line_and_column(text, start.saturating_add(failure.offset));
                format!(
                    "{} at line {line} column {}",
                    failure.message,
                    column.saturating_add(1)
                )
            })?;
        let replacement_text = normalized_string.as_ref().map_or_else(
            || format!("\"{}\"", unique_marker(text, index)),
            |normalized| normalized.text.clone(),
        );
        masked.push_str(&replacement_text);
        let masked_end = masked.len();
        source_map.push(MaskedSourceRange {
            original_start: start,
            original_end: end,
            masked_start,
            masked_end,
        });
        match replacement.kind {
            ReplacementKind::Number => masked_numbers.push(MaskedNumberSource {
                marker: replacement_text.trim_matches('"').to_owned(),
                original,
            }),
            ReplacementKind::String => {
                let normalized = normalized_string
                    .ok_or_else(|| "normalized string replacement is missing".to_owned())?;
                masked_strings.push(MaskedStringSource {
                    masked: replacement_text,
                    original,
                    unrepresentable: normalized.unrepresentable,
                });
            }
        }
        cursor = end;
    }
    masked.push_str(
        text.get(cursor..)
            .ok_or_else(|| "number suffix range is not valid UTF-8".to_owned())?,
    );
    Ok(MaskedDocument {
        text: masked,
        numbers: masked_numbers,
        strings: masked_strings,
        source_map,
    })
}

#[derive(Clone, Copy)]
enum ReplacementKind {
    Number,
    String,
}

struct Replacement {
    original_start: usize,
    original_end: usize,
    kind: ReplacementKind,
}

impl Replacement {
    const fn number(original_start: usize, original_end: usize) -> Self {
        Self {
            original_start,
            original_end,
            kind: ReplacementKind::Number,
        }
    }

    fn source_range(&self) -> Result<(usize, usize), String> {
        match self.kind {
            ReplacementKind::Number => Ok((
                self.original_start
                    .checked_sub(1)
                    .ok_or_else(|| "number range starts outside the JSONC document".to_owned())?,
                self.original_end
                    .checked_sub(1)
                    .ok_or_else(|| "number range ends outside the JSONC document".to_owned())?,
            )),
            ReplacementKind::String => Ok((self.original_start, self.original_end)),
        }
    }

    const fn string(original_start: usize, original_end: usize) -> Self {
        Self {
            original_start,
            original_end,
            kind: ReplacementKind::String,
        }
    }
}

fn collect_string_replacements(
    node: Node<'_>,
    source: &str,
    replacements: &mut Vec<Replacement>,
) -> Result<(), String> {
    if node.kind() == "string" {
        let raw = source
            .get(node.byte_range())
            .ok_or_else(|| "string range is not valid UTF-8".to_owned())?;
        if raw.starts_with('"') {
            replacements.push(Replacement::string(
                node.start_byte()
                    .checked_sub(1)
                    .ok_or_else(|| "string starts outside the JSONC document".to_owned())?,
                node.end_byte()
                    .checked_sub(1)
                    .ok_or_else(|| "string ends outside the JSONC document".to_owned())?,
            ));
        }
        return Ok(());
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_string_replacements(child, source, replacements)?;
    }
    Ok(())
}

struct NormalizedString {
    text: String,
    unrepresentable: bool,
}

struct StringNormalizationFailure {
    message: String,
    offset: usize,
}

fn normalize_extended_string(
    original: &str,
    surrogate_marker: &str,
) -> Result<NormalizedString, StringNormalizationFailure> {
    let bytes = original.as_bytes();
    let mut normalized = String::with_capacity(original.len());
    let mut index = 0;
    let mut cursor = 0;
    let mut unrepresentable = false;
    while let Some(&byte) = bytes.get(index) {
        if byte != b'\\' {
            index = index.saturating_add(1);
            continue;
        }
        normalized.push_str(original.get(cursor..index).ok_or_else(|| {
            StringNormalizationFailure {
                message: "string escape boundary is not valid UTF-8".to_owned(),
                offset: index,
            }
        })?);
        index = append_normalized_escape(
            original,
            index,
            surrogate_marker,
            &mut normalized,
            &mut unrepresentable,
        )
        .map_err(|message| StringNormalizationFailure {
            message,
            offset: index,
        })?;
        cursor = index;
    }
    normalized.push_str(
        original
            .get(cursor..)
            .ok_or_else(|| StringNormalizationFailure {
                message: "string suffix boundary is not valid UTF-8".to_owned(),
                offset: cursor,
            })?,
    );
    Ok(NormalizedString {
        text: normalized,
        unrepresentable,
    })
}

fn append_normalized_escape(
    original: &str,
    index: usize,
    surrogate_marker: &str,
    normalized: &mut String,
    unrepresentable: &mut bool,
) -> Result<usize, String> {
    let bytes = original.as_bytes();
    let escaped = original
        .get(index.saturating_add(1)..)
        .and_then(|suffix| suffix.chars().next())
        .ok_or_else(|| "unterminated string escape".to_owned())?;
    match escaped {
        'x' => {
            let digits = original
                .get(index.saturating_add(2)..index.saturating_add(4))
                .ok_or_else(|| "incomplete hexadecimal string escape".to_owned())?;
            if !digits.bytes().all(|digit| digit.is_ascii_hexdigit()) {
                return Err("invalid hexadecimal string escape".to_owned());
            }
            normalized.push_str("\\u00");
            normalized.push_str(digits);
            Ok(index.saturating_add(4))
        }
        'v' => {
            normalized.push_str("\\u000b");
            Ok(index.saturating_add(2))
        }
        '0' if bytes
            .get(index.saturating_add(2))
            .is_none_or(|next| !next.is_ascii_digit()) =>
        {
            normalized.push_str("\\u0000");
            Ok(index.saturating_add(2))
        }
        '0'..='7' => Err("octal string escapes are not allowed".to_owned()),
        '8' | '9' => Err("decimal digit string escapes are not allowed".to_owned()),
        '\n' => Ok(index.saturating_add(2)),
        '\r' => Ok(
            index.saturating_add(if bytes.get(index.saturating_add(2)) == Some(&b'\n') {
                3
            } else {
                2
            }),
        ),
        'u' if bytes.get(index.saturating_add(2)) == Some(&b'{') => append_code_point_escape(
            original,
            index,
            surrogate_marker,
            normalized,
            unrepresentable,
        ),
        'u' => append_standard_unicode_escape(
            original,
            index,
            surrogate_marker,
            normalized,
            unrepresentable,
        ),
        '\u{2028}' | '\u{2029}' => Ok(index.saturating_add(1).saturating_add(escaped.len_utf8())),
        '\'' => {
            normalized.push('\'');
            Ok(index.saturating_add(2))
        }
        identity
            if !matches!(
                identity,
                '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' | 'u'
            ) =>
        {
            normalized.push(identity);
            Ok(index.saturating_add(1).saturating_add(identity.len_utf8()))
        }
        _ => {
            normalized.push('\\');
            normalized.push(escaped);
            Ok(index.saturating_add(1).saturating_add(escaped.len_utf8()))
        }
    }
}

fn append_code_point_escape(
    original: &str,
    index: usize,
    surrogate_marker: &str,
    normalized: &mut String,
    unrepresentable: &mut bool,
) -> Result<usize, String> {
    let digits_start = index.saturating_add(3);
    let close_offset = original
        .as_bytes()
        .get(digits_start..)
        .and_then(|suffix| suffix.iter().position(|candidate| *candidate == b'}'))
        .ok_or_else(|| "unterminated Unicode code point escape".to_owned())?;
    let digits_end = digits_start.saturating_add(close_offset);
    let digits = original
        .get(digits_start..digits_end)
        .ok_or_else(|| "Unicode escape boundary is not valid UTF-8".to_owned())?;
    let value = u32::from_str_radix(digits, 16)
        .ok()
        .filter(|value| *value <= 0x10_ffff)
        .ok_or_else(|| "invalid Unicode code point escape".to_owned())?;
    if (0xd800..=0xdfff).contains(&value) {
        append_surrogate_marker(normalized, surrogate_marker, value)?;
        *unrepresentable = true;
    } else if value <= 0xffff {
        append_utf16_escape(normalized, value)?;
    } else {
        let offset = value
            .checked_sub(0x1_0000)
            .ok_or_else(|| "invalid Unicode code point escape".to_owned())?;
        append_utf16_escape(normalized, 0xd800_u32.saturating_add(offset >> 10))?;
        append_utf16_escape(normalized, 0xdc00_u32.saturating_add(offset & 0x3ff))?;
    }
    Ok(digits_end.saturating_add(1))
}

fn append_standard_unicode_escape(
    original: &str,
    index: usize,
    surrogate_marker: &str,
    normalized: &mut String,
    unrepresentable: &mut bool,
) -> Result<usize, String> {
    let digits = original
        .get(index.saturating_add(2)..index.saturating_add(6))
        .ok_or_else(|| "incomplete Unicode string escape".to_owned())?;
    let value =
        u32::from_str_radix(digits, 16).map_err(|_| "invalid Unicode string escape".to_owned())?;
    if (0xd800..=0xdbff).contains(&value)
        && let Some(low_digits) = original
            .get(index.saturating_add(8)..index.saturating_add(12))
            .filter(|_| {
                original.get(index.saturating_add(6)..index.saturating_add(8)) == Some("\\u")
            })
        && let Ok(low) = u32::from_str_radix(low_digits, 16)
        && (0xdc00..=0xdfff).contains(&low)
    {
        normalized.push_str(
            original
                .get(index..index.saturating_add(12))
                .ok_or_else(|| "Unicode surrogate pair boundary is not valid UTF-8".to_owned())?,
        );
        Ok(index.saturating_add(12))
    } else if (0xd800..=0xdfff).contains(&value) {
        append_surrogate_marker(normalized, surrogate_marker, value)?;
        *unrepresentable = true;
        Ok(index.saturating_add(6))
    } else {
        normalized.push_str("\\u");
        Ok(index.saturating_add(2))
    }
}

fn append_surrogate_marker(
    normalized: &mut String,
    surrogate_marker: &str,
    value: u32,
) -> Result<(), String> {
    normalized.push_str(surrogate_marker);
    std::fmt::Write::write_fmt(normalized, format_args!("{value:04x}"))
        .map_err(|error| error.to_string())
}

fn append_utf16_escape(normalized: &mut String, value: u32) -> Result<(), String> {
    std::fmt::Write::write_fmt(normalized, format_args!("\\u{value:04x}"))
        .map_err(|error| error.to_string())
}

fn collect_unsupported_numbers(
    node: Node<'_>,
    source: &str,
    parser_options: &ParseOptions,
    ranges: &mut Vec<(usize, usize)>,
) -> Result<(), String> {
    if node.kind() == "number" {
        if number_is_inside_property_name(node) {
            return Ok(());
        }
        let number = source
            .get(node.byte_range())
            .ok_or_else(|| "number token is not valid UTF-8".to_owned())?;
        if number.contains('_') && !numeric_separators_are_valid(number) {
            return Err(format!("invalid numeric separator in `{number}`"));
        }
        if !configured_number_parses(number, parser_options)
            && is_extended_json_number(number, parser_options)
        {
            let range = signed_number_range(node, source, parser_options);
            ranges.push(range);
        }
        return Ok(());
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_unsupported_numbers(child, source, parser_options, ranges)?;
    }
    Ok(())
}

fn number_is_inside_property_name(node: Node<'_>) -> bool {
    let mut ancestor = node.parent();
    while let Some(parent) = ancestor {
        if parent.kind() == "pair" {
            return parent.child_by_field_name("key").is_some_and(|key| {
                key.start_byte() <= node.start_byte() && node.end_byte() <= key.end_byte()
            });
        }
        ancestor = parent.parent();
    }
    false
}

fn is_extended_json_number(number: &str, options: &ParseOptions) -> bool {
    if number.ends_with('n') {
        return false;
    }
    let unsigned = number
        .strip_prefix('-')
        .or_else(|| number.strip_prefix('+'))
        .unwrap_or(number);
    if !numeric_separators_are_valid(unsigned) {
        return false;
    }
    if unsigned.starts_with("0x") || unsigned.starts_with("0X") {
        return options.allow_hexadecimal_numbers;
    }
    !(unsigned.starts_with('0') && unsigned.as_bytes().get(1).is_some_and(u8::is_ascii_digit))
}

pub(super) fn numeric_separators_are_valid(unsigned: &str) -> bool {
    if unsigned.starts_with("0_") {
        return false;
    }
    let hexadecimal = unsigned.starts_with("0x") || unsigned.starts_with("0X");
    let bytes = unsigned.as_bytes();
    bytes.iter().enumerate().all(|(index, byte)| {
        if *byte != b'_' {
            return true;
        }
        let valid_digit = |candidate: u8| {
            if hexadecimal {
                candidate.is_ascii_hexdigit()
            } else {
                candidate.is_ascii_digit()
            }
        };
        index
            .checked_sub(1)
            .and_then(|previous| bytes.get(previous))
            .copied()
            .is_some_and(valid_digit)
            && bytes
                .get(index.saturating_add(1))
                .copied()
                .is_some_and(valid_digit)
    })
}

fn signed_number_range(
    node: Node<'_>,
    source: &str,
    parser_options: &ParseOptions,
) -> (usize, usize) {
    let Some(parent) = node
        .parent()
        .filter(|parent| parent.kind() == "unary_expression")
    else {
        return (node.start_byte(), node.end_byte());
    };
    let Some(operator) = parent.child_by_field_name("operator") else {
        return (node.start_byte(), node.end_byte());
    };
    let operator_text = source.get(operator.byte_range()).unwrap_or_default();
    if operator_text == "-" || (operator_text == "+" && parser_options.allow_unary_plus_numbers) {
        (parent.start_byte(), parent.end_byte())
    } else {
        (node.start_byte(), node.end_byte())
    }
}

fn configured_number_parses(number: &str, options: &ParseOptions) -> bool {
    CstRootNode::parse(&format!("{{\"value\":{number}}}"), options).is_ok()
}

fn unique_marker(text: &str, index: usize) -> String {
    let mut marker = format!("__AQC_JSON_NUMBER_{index}__");
    while text.contains(&marker) {
        marker.push('_');
    }
    marker
}
