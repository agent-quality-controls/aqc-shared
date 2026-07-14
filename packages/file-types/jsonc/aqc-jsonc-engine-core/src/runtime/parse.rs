use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, Severity};
use jsonc_parser::ParseOptions;
use jsonc_parser::cst::{CstNode, CstObject, CstRootNode};
use tree_sitter::{Node, Parser};

use crate::types::MaskedNumber;
use crate::{JsoncObject, JsoncParseOptions};

#[must_use]
pub fn parse_object_or_report(
    current_bytes: Option<&[u8]>,
    file_label: &str,
    options: JsoncParseOptions,
) -> (Option<JsoncObject>, Vec<Finding>) {
    let parser_options = parser_options(options);
    let parsed = current_bytes.map_or_else(
        || parse_document("{}\n", &parser_options, options),
        |bytes| {
            std::str::from_utf8(bytes)
                .map_err(|error| error.to_string())
                .and_then(|text| parse_document(text, &parser_options, options))
        },
    );
    let parsed = parsed.and_then(|document| {
        let ParsedDocument {
            root,
            masked_numbers,
            utf8_bom,
        } = document;
        validate_root(root).map(|root| ParsedDocument {
            root,
            masked_numbers,
            utf8_bom,
        })
    });
    match parsed {
        Ok(document) => {
            let (masked_numbers, masked_scalars) =
                bind_masked_numbers(&document.root, document.masked_numbers);
            (
                Some(JsoncObject {
                    root: document.root,
                    masked_numbers,
                    masked_scalars,
                    utf8_bom: document.utf8_bom,
                }),
                Vec::new(),
            )
        }
        Err(error) => (
            None,
            vec![Finding::ParseError {
                message: format!("{file_label} is not a valid JSONC object: {error}"),
                severity: Severity::Error,
            }],
        ),
    }
}

struct ParsedDocument {
    root: CstRootNode,
    masked_numbers: Vec<(String, String)>,
    utf8_bom: bool,
}

fn parse_document(
    text: &str,
    parser_options: &ParseOptions,
    options: JsoncParseOptions,
) -> Result<ParsedDocument, String> {
    let (text, utf8_bom) = text
        .strip_prefix('\u{feff}')
        .map_or((text, false), |text| (text, true));
    if utf8_bom && !options.allow_utf8_bom {
        return Err("UTF-8 BOM is not allowed".to_owned());
    }
    if let Ok(root) = CstRootNode::parse(text, parser_options) {
        return Ok(ParsedDocument {
            root,
            masked_numbers: Vec::new(),
            utf8_bom,
        });
    }
    if !options.allow_extended_json_numbers {
        return CstRootNode::parse(text, parser_options)
            .map(|root| ParsedDocument {
                root,
                masked_numbers: Vec::new(),
                utf8_bom,
            })
            .map_err(|error| error.to_string());
    }
    let (masked, masked_numbers) = mask_extended_json_numbers(text, parser_options)?;
    CstRootNode::parse(&masked, parser_options)
        .map(|root| ParsedDocument {
            root,
            masked_numbers,
            utf8_bom,
        })
        .map_err(|error| error.to_string())
}

fn mask_extended_json_numbers(
    text: &str,
    parser_options: &ParseOptions,
) -> Result<(String, Vec<(String, String)>), String> {
    let wrapped = format!("({text})");
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_javascript::LANGUAGE.into())
        .map_err(|error| error.to_string())?;
    let tree = parser
        .parse(&wrapped, None)
        .ok_or_else(|| "JavaScript number parser returned no syntax tree".to_owned())?;
    if tree.root_node().has_error() {
        return Err("input is not valid JSONC syntax".to_owned());
    }
    let mut ranges = Vec::new();
    collect_unsupported_numbers(tree.root_node(), &wrapped, parser_options, &mut ranges)?;
    if ranges.is_empty() {
        return Err("input is not valid JSONC syntax".to_owned());
    }
    ranges.sort_unstable_by_key(|(start, _)| *start);
    ranges.dedup();
    let mut masked = text.to_owned();
    let mut masked_numbers = Vec::with_capacity(ranges.len());
    for (index, (wrapped_start, wrapped_end)) in ranges.into_iter().enumerate().rev() {
        let start = wrapped_start
            .checked_sub(1)
            .ok_or_else(|| "number range starts outside the JSONC document".to_owned())?;
        let end = wrapped_end
            .checked_sub(1)
            .ok_or_else(|| "number range ends outside the JSONC document".to_owned())?;
        let original = text
            .get(start..end)
            .ok_or_else(|| "number range is not valid UTF-8".to_owned())?
            .to_owned();
        let marker = unique_marker(text, index);
        masked.replace_range(start..end, &format!("\"{marker}\""));
        masked_numbers.push((marker, original));
    }
    masked_numbers.reverse();
    Ok((masked, masked_numbers))
}

fn collect_unsupported_numbers(
    node: Node<'_>,
    source: &str,
    parser_options: &ParseOptions,
    ranges: &mut Vec<(usize, usize)>,
) -> Result<(), String> {
    if node.kind() == "number" {
        let number = source
            .get(node.byte_range())
            .ok_or_else(|| "number token is not valid UTF-8".to_owned())?;
        if !jsonc_number_parses(number, parser_options) && is_extended_json_number(number) {
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

fn is_extended_json_number(number: &str) -> bool {
    if number.ends_with('n') {
        return false;
    }
    let unsigned = number
        .strip_prefix('-')
        .or_else(|| number.strip_prefix('+'))
        .unwrap_or(number);
    !(unsigned.starts_with('0') && unsigned.as_bytes().get(1).is_some_and(u8::is_ascii_digit))
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

fn jsonc_number_parses(number: &str, options: &ParseOptions) -> bool {
    CstRootNode::parse(&format!("{{\"value\":{number}}}"), options).is_ok()
}

fn unique_marker(text: &str, index: usize) -> String {
    let mut marker = format!("__AQC_JSONC_NUMBER_{index}__");
    while text.contains(&marker) {
        marker.push('_');
    }
    marker
}

fn bind_masked_numbers(
    root: &CstRootNode,
    originals: Vec<(String, String)>,
) -> (Vec<MaskedNumber>, BTreeMap<Vec<String>, String>) {
    let originals = originals.into_iter().collect::<BTreeMap<_, _>>();
    let mut masked_numbers = Vec::new();
    let mut masked_scalars = BTreeMap::new();
    if let Some(object) = root.object_value() {
        bind_object_numbers(
            &object,
            &originals,
            &mut Vec::new(),
            true,
            &mut masked_numbers,
            &mut masked_scalars,
        );
    }
    (masked_numbers, masked_scalars)
}

fn bind_object_numbers(
    object: &CstObject,
    originals: &BTreeMap<String, String>,
    path: &mut Vec<String>,
    addressable: bool,
    masked_numbers: &mut Vec<MaskedNumber>,
    masked_scalars: &mut BTreeMap<Vec<String>, String>,
) {
    for property in object.properties() {
        let Some(name) = property.name().and_then(|name| name.decoded_value().ok()) else {
            continue;
        };
        let Some(value) = property.value() else {
            continue;
        };
        path.push(name);
        bind_node_numbers(
            &value,
            originals,
            path,
            addressable,
            masked_numbers,
            masked_scalars,
        );
        let _ = path.pop();
    }
}

fn bind_node_numbers(
    node: &CstNode,
    originals: &BTreeMap<String, String>,
    path: &mut Vec<String>,
    addressable: bool,
    masked_numbers: &mut Vec<MaskedNumber>,
    masked_scalars: &mut BTreeMap<Vec<String>, String>,
) {
    if let Some(literal) = node.as_string_lit()
        && let Ok(marker) = literal.decoded_value()
        && let Some(original) = originals.get(&marker)
    {
        masked_numbers.push(MaskedNumber {
            literal,
            marker,
            original: original.clone(),
        });
        if addressable {
            let _ = masked_scalars.insert(path.clone(), original.clone());
        }
        return;
    }
    if let Some(object) = node.as_object() {
        bind_object_numbers(
            &object,
            originals,
            path,
            addressable,
            masked_numbers,
            masked_scalars,
        );
    } else if let Some(array) = node.as_array() {
        for element in array.elements() {
            bind_node_numbers(
                &element,
                originals,
                path,
                false,
                masked_numbers,
                masked_scalars,
            );
        }
    }
}

const fn parser_options(options: JsoncParseOptions) -> ParseOptions {
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

fn validate_root(root: CstRootNode) -> Result<CstRootNode, String> {
    let object = root
        .object_value()
        .ok_or_else(|| "root value must be an object".to_owned())?;
    reject_duplicates(&object)?;
    Ok(root)
}

fn reject_duplicates(object: &CstObject) -> Result<(), String> {
    let mut names = BTreeSet::new();
    for property in object.properties() {
        let name = property
            .name()
            .ok_or_else(|| "object property is missing a name".to_owned())?
            .decoded_value()
            .map_err(|error| format!("invalid object property name: {error:?}"))?;
        if !names.insert(name.clone()) {
            return Err(format!("duplicate object member `{name}`"));
        }
        if let Some(value) = property.value() {
            reject_nested(&value)?;
        }
    }
    Ok(())
}

fn reject_nested(node: &CstNode) -> Result<(), String> {
    if let Some(object) = node.as_object() {
        reject_duplicates(&object)?;
    } else if let Some(array) = node.as_array() {
        for element in array.elements() {
            reject_nested(&element)?;
        }
    }
    Ok(())
}
