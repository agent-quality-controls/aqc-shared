//! Root mapping parser and structural validation.

use std::collections::BTreeSet;
use std::str::FromStr;

use aqc_file_engine_core::{Finding, Severity};
use yaml_edit::{AnchorRegistry, Document, DocumentResolvedExt, ScalarType, ScalarValue, YamlNode};

use crate::types::ParsedYamlMapping;

/// Parses optional YAML bytes as a duplicate-safe root mapping.
///
/// # Errors
///
/// Returns one parse or root-shape finding when the input is not a valid,
/// unambiguous YAML mapping.
pub fn parse_yaml_mapping(
    current_bytes: Option<&[u8]>,
    file_name: &str,
) -> Result<ParsedYamlMapping, Finding> {
    let Some(bytes) = current_bytes else {
        return Ok(empty_mapping());
    };
    let text =
        std::str::from_utf8(bytes).map_err(|error| parse_error(file_name, &error.to_string()))?;
    let document =
        Document::from_str(text).map_err(|error| parse_error(file_name, &error.to_string()))?;
    validate_root(&document, file_name)?;
    Ok(ParsedYamlMapping {
        document,
        original: Some(bytes.to_vec()),
        dirty: core::cell::Cell::new(false),
    })
}

pub(crate) fn root_keys(document: &Document) -> Vec<String> {
    let Some(mapping) = document.as_mapping() else {
        return Vec::new();
    };
    let registry = document.build_anchor_registry();
    mapping
        .keys()
        .filter_map(|node| crate::runtime::decode::decode_mapping_key(&node, &registry).ok())
        .collect()
}

fn empty_mapping() -> ParsedYamlMapping {
    ParsedYamlMapping {
        document: Document::new_mapping(),
        original: None,
        dirty: core::cell::Cell::new(false),
    }
}

fn validate_root(document: &Document, file_name: &str) -> Result<(), Finding> {
    let Some(mapping) = document.as_mapping() else {
        return Err(parse_error(file_name, "root must be a mapping"));
    };
    let registry = document.build_anchor_registry();
    validate_mapping(&mapping, true, file_name, &registry)
}

fn validate_mapping(
    mapping: &yaml_edit::Mapping,
    root: bool,
    file_name: &str,
    registry: &AnchorRegistry,
) -> Result<(), Finding> {
    let mut keys = BTreeSet::new();
    for (key, value) in mapping.iter() {
        let identity = mapping_key_identity(&key, registry)
            .map_err(|detail| parse_error(file_name, detail))?;
        if root && identity.0 != "string" {
            return Err(parse_error(file_name, "root mapping keys must be strings"));
        }
        if !keys.insert(identity.clone()) {
            return Err(parse_error(
                file_name,
                &format!("duplicate mapping key `{}`", identity.1),
            ));
        }
        validate_node(&value, file_name, registry)?;
    }
    Ok(())
}

fn mapping_key_identity(
    key: &YamlNode,
    registry: &AnchorRegistry,
) -> Result<(String, String), &'static str> {
    match key {
        YamlNode::Scalar(scalar) => Ok(scalar_key_identity(scalar)),
        YamlNode::TaggedNode(tagged)
            if tagged
                .tag()
                .is_some_and(|tag| tag == "!!str" || tag == "tag:yaml.org,2002:str") =>
        {
            tagged
                .value()
                .map(|value| ("string".to_owned(), value.as_string()))
                .ok_or("tagged mapping keys must be scalar strings")
        }
        YamlNode::Alias(alias) => registry
            .resolve(&alias.name())
            .and_then(|value| Document::from_str(&value.to_string()).ok())
            .and_then(|document| document.as_scalar())
            .filter(is_string_scalar)
            .map(|scalar| ("string".to_owned(), scalar.as_string()))
            .ok_or("aliased mapping keys must resolve to scalar strings"),
        YamlNode::TaggedNode(_) => Err("tagged mapping keys must use the YAML string tag"),
        YamlNode::Mapping(_) | YamlNode::Sequence(_) => Err("mapping keys must be strings"),
    }
}

fn scalar_key_identity(scalar: &yaml_edit::Scalar) -> (String, String) {
    let value = ScalarValue::from_scalar(scalar);
    if is_string_scalar(scalar) {
        ("string".to_owned(), scalar.as_string())
    } else if let Some(boolean) = strict_boolean(scalar) {
        ("boolean".to_owned(), boolean.to_string())
    } else if let Some(integer) = scalar.as_i64() {
        ("integer".to_owned(), integer.to_string())
    } else {
        (format!("{:?}", value.scalar_type()), scalar.as_string())
    }
}

fn validate_node(
    node: &YamlNode,
    file_name: &str,
    registry: &AnchorRegistry,
) -> Result<(), Finding> {
    match node {
        YamlNode::Mapping(child) => validate_mapping(child, false, file_name, registry),
        YamlNode::Sequence(sequence) => {
            for item in sequence.values() {
                validate_node(&item, file_name, registry)?;
            }
            Ok(())
        }
        YamlNode::Scalar(_) | YamlNode::Alias(_) | YamlNode::TaggedNode(_) => Ok(()),
    }
}

pub(crate) fn strict_boolean(scalar: &yaml_edit::Scalar) -> Option<bool> {
    if scalar.is_quoted() {
        return None;
    }
    match scalar.as_string().as_str() {
        "true" | "True" | "TRUE" => Some(true),
        "false" | "False" | "FALSE" => Some(false),
        _ => None,
    }
}

pub(crate) fn is_string_scalar(scalar: &yaml_edit::Scalar) -> bool {
    if scalar.is_quoted() {
        return true;
    }
    match ScalarValue::from_scalar(scalar).scalar_type() {
        ScalarType::Integer | ScalarType::Float | ScalarType::Null => false,
        ScalarType::Boolean => strict_boolean(scalar).is_none(),
        ScalarType::String | ScalarType::Timestamp | ScalarType::Regex | ScalarType::Binary => true,
    }
}

fn parse_error(file_name: &str, detail: &str) -> Finding {
    Finding::ParseError {
        message: format!("failed to parse {file_name}: {detail}"),
        severity: Severity::Error,
    }
}
