//! Merged-field lookup and strict pnpm-compatible YAML decoding.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use yaml_edit::{AnchorRegistry, Document, DocumentResolvedExt, YamlNode};

use crate::runtime::parse::{is_string_scalar, strict_boolean};
use crate::types::{YamlFieldError, YamlFieldValue};

pub(crate) fn read_field(
    document: &Document,
    key: &str,
) -> Result<Option<YamlFieldValue>, YamlFieldError> {
    let mapping = document.as_mapping().ok_or(YamlFieldError::WrongShape)?;
    let registry = document.build_anchor_registry();
    effective_mapping_value(&mapping, key, &registry)?
        .map(|node| decode_node(node, &registry))
        .transpose()
}

pub(crate) fn effective_root_keys(document: &Document) -> Result<Vec<String>, YamlFieldError> {
    let mapping = document.as_mapping().ok_or(YamlFieldError::WrongShape)?;
    let registry = document.build_anchor_registry();
    effective_mapping_entries(&mapping, &registry)?
        .into_iter()
        .map(|(node, _)| node)
        .map(|node| decode_mapping_key(&node, &registry))
        .collect()
}

fn decode_node(
    node: YamlNode,
    registry: &AnchorRegistry,
) -> Result<YamlFieldValue, YamlFieldError> {
    match node {
        YamlNode::Scalar(scalar) => decode_scalar(&scalar),
        YamlNode::Sequence(sequence) => decode_sequence(&sequence, registry),
        YamlNode::Mapping(mapping) => decode_mapping(&mapping, registry),
        YamlNode::Alias(alias) => decode_alias(&alias.name(), registry),
        YamlNode::TaggedNode(tagged) => decode_tagged(&tagged),
    }
}

fn decode_scalar(scalar: &yaml_edit::Scalar) -> Result<YamlFieldValue, YamlFieldError> {
    if let Some(value) = strict_boolean(scalar) {
        return Ok(YamlFieldValue::Boolean(value));
    }
    if let Some(value) = scalar.as_i64() {
        return u64::try_from(value)
            .map(YamlFieldValue::Integer)
            .map_err(|_| YamlFieldError::WrongShape);
    }
    if is_string_scalar(scalar) {
        Ok(YamlFieldValue::String(scalar.as_string()))
    } else {
        Err(YamlFieldError::WrongShape)
    }
}

fn decode_sequence(
    sequence: &yaml_edit::Sequence,
    registry: &AnchorRegistry,
) -> Result<YamlFieldValue, YamlFieldError> {
    let values = sequence
        .values()
        .map(|node| match node {
            YamlNode::Scalar(scalar) if is_string_scalar(&scalar) => Ok(scalar.as_string()),
            YamlNode::Alias(alias) => decode_alias_string(&alias.name(), registry),
            YamlNode::TaggedNode(tagged) => decode_tagged_string(&tagged),
            YamlNode::Mapping(_) | YamlNode::Sequence(_) | YamlNode::Scalar(_) => {
                Err(YamlFieldError::WrongShape)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(YamlFieldValue::StringSequence(values))
}

fn decode_mapping(
    mapping: &yaml_edit::Mapping,
    registry: &AnchorRegistry,
) -> Result<YamlFieldValue, YamlFieldError> {
    let mut values = BTreeMap::new();
    for (key, value) in effective_mapping_entries(mapping, registry)? {
        let key = decode_mapping_key(&key, registry)?;
        let value = match value {
            YamlNode::Scalar(scalar) => {
                strict_boolean(&scalar).ok_or(YamlFieldError::WrongShape)?
            }
            YamlNode::TaggedNode(tagged) => decode_tagged_bool(&tagged)?,
            YamlNode::Alias(alias) => decode_alias_bool(&alias.name(), registry)?,
            YamlNode::Mapping(_) | YamlNode::Sequence(_) => {
                return Err(YamlFieldError::WrongShape);
            }
        };
        let _ = values.insert(key, value);
    }
    Ok(YamlFieldValue::StringBooleanMapping(values))
}

fn resolve_alias_document(
    name: &str,
    registry: &AnchorRegistry,
) -> Result<Document, YamlFieldError> {
    let target = registry
        .resolve(name)
        .ok_or(YamlFieldError::UnresolvedAlias)?;
    Document::from_str(&target.to_string()).map_err(|_| YamlFieldError::WrongShape)
}

fn decode_alias(name: &str, registry: &AnchorRegistry) -> Result<YamlFieldValue, YamlFieldError> {
    let document = resolve_alias_document(name, registry)?;
    if let Some(scalar) = document.as_scalar() {
        return decode_scalar(&scalar);
    }
    if let Some(sequence) = document.as_sequence() {
        return decode_sequence(&sequence, registry);
    }
    document.as_mapping().map_or_else(
        || Err(YamlFieldError::WrongShape),
        |mapping| decode_mapping(&mapping, registry),
    )
}

fn decode_alias_string(name: &str, registry: &AnchorRegistry) -> Result<String, YamlFieldError> {
    let document = resolve_alias_document(name, registry)?;
    document
        .as_scalar()
        .filter(is_string_scalar)
        .map(|scalar| scalar.as_string())
        .ok_or(YamlFieldError::WrongShape)
}

fn decode_alias_bool(name: &str, registry: &AnchorRegistry) -> Result<bool, YamlFieldError> {
    let document = resolve_alias_document(name, registry)?;
    document
        .as_scalar()
        .and_then(|scalar| strict_boolean(&scalar))
        .ok_or(YamlFieldError::WrongShape)
}

fn decode_tagged(tagged: &yaml_edit::TaggedNode) -> Result<YamlFieldValue, YamlFieldError> {
    match normalized_tag(tagged).as_deref() {
        Some("bool") => decode_tagged_bool(tagged).map(YamlFieldValue::Boolean),
        Some("str") => decode_tagged_string(tagged).map(YamlFieldValue::String),
        Some("int") => decode_tagged_integer(tagged).map(YamlFieldValue::Integer),
        _ => Err(YamlFieldError::UnknownTag),
    }
}

fn decode_tagged_bool(tagged: &yaml_edit::TaggedNode) -> Result<bool, YamlFieldError> {
    if normalized_tag(tagged).as_deref() != Some("bool") {
        return Err(YamlFieldError::UnknownTag);
    }
    tagged
        .value()
        .and_then(|value| strict_boolean(&value))
        .ok_or(YamlFieldError::WrongShape)
}

fn decode_tagged_string(tagged: &yaml_edit::TaggedNode) -> Result<String, YamlFieldError> {
    if normalized_tag(tagged).as_deref() != Some("str") {
        return Err(YamlFieldError::UnknownTag);
    }
    tagged
        .value()
        .map(|value| value.as_string())
        .ok_or(YamlFieldError::WrongShape)
}

fn decode_tagged_integer(tagged: &yaml_edit::TaggedNode) -> Result<u64, YamlFieldError> {
    if normalized_tag(tagged).as_deref() != Some("int") {
        return Err(YamlFieldError::UnknownTag);
    }
    tagged
        .value()
        .and_then(|value| value.as_i64())
        .and_then(|value| u64::try_from(value).ok())
        .ok_or(YamlFieldError::WrongShape)
}

fn normalized_tag(tagged: &yaml_edit::TaggedNode) -> Option<String> {
    tagged.tag().map(|tag| {
        tag.strip_prefix("!!")
            .or_else(|| tag.strip_prefix("tag:yaml.org,2002:"))
            .unwrap_or(&tag)
            .to_owned()
    })
}

fn validate_mapping_merge_sources(
    mapping: &yaml_edit::Mapping,
    registry: &AnchorRegistry,
) -> Result<(), YamlFieldError> {
    for (key, value) in mapping.iter() {
        if is_merge_key(&key) {
            validate_merge_node(&value, registry)?;
        }
    }
    Ok(())
}

fn validate_merge_node(
    node: &YamlNode,
    registry: &yaml_edit::AnchorRegistry,
) -> Result<(), YamlFieldError> {
    match node {
        YamlNode::Alias(alias) => validate_merge_alias(&alias.name(), registry),
        YamlNode::Sequence(sequence) => {
            for item in sequence.values() {
                validate_merge_node(&item, registry)?;
            }
            Ok(())
        }
        YamlNode::Mapping(mapping) => validate_mapping_merge_sources(mapping, registry),
        YamlNode::Scalar(_) | YamlNode::TaggedNode(_) => Err(YamlFieldError::InvalidMergeSource),
    }
}

fn effective_mapping_value(
    mapping: &yaml_edit::Mapping,
    key: &str,
    registry: &AnchorRegistry,
) -> Result<Option<YamlNode>, YamlFieldError> {
    effective_mapping_value_inner(mapping, key, registry, &mut BTreeSet::new())
}

fn effective_mapping_value_inner(
    mapping: &yaml_edit::Mapping,
    key: &str,
    registry: &AnchorRegistry,
    active_anchors: &mut BTreeSet<String>,
) -> Result<Option<YamlNode>, YamlFieldError> {
    for (candidate, value) in mapping.iter() {
        if !is_merge_key(&candidate) && decode_mapping_key(&candidate, registry)? == key {
            return Ok(Some(value));
        }
    }
    validate_mapping_merge_sources(mapping, registry)?;
    for (anchor, source) in merge_source_mappings(mapping, registry)? {
        if anchor
            .as_ref()
            .is_some_and(|name| !active_anchors.insert(name.clone()))
        {
            return Err(YamlFieldError::InvalidMergeSource);
        }
        let resolved = effective_mapping_value_inner(&source, key, registry, active_anchors);
        if let Some(name) = anchor {
            let _ = active_anchors.remove(&name);
        }
        if let Some(value) = resolved? {
            return Ok(Some(value));
        }
    }
    Ok(None)
}

fn effective_mapping_entries(
    mapping: &yaml_edit::Mapping,
    registry: &AnchorRegistry,
) -> Result<Vec<(YamlNode, YamlNode)>, YamlFieldError> {
    effective_mapping_entries_inner(mapping, registry, &mut BTreeSet::new())
}

fn effective_mapping_entries_inner(
    mapping: &yaml_edit::Mapping,
    registry: &AnchorRegistry,
    active_anchors: &mut BTreeSet<String>,
) -> Result<Vec<(YamlNode, YamlNode)>, YamlFieldError> {
    validate_mapping_merge_sources(mapping, registry)?;
    let mut entries = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (key, value) in mapping.iter() {
        let key_text = decode_mapping_key(&key, registry)?;
        if !is_merge_key(&key) {
            let _ = seen.insert(key_text);
            entries.push((key, value));
        }
    }
    for (anchor, source) in merge_source_mappings(mapping, registry)? {
        if anchor
            .as_ref()
            .is_some_and(|name| !active_anchors.insert(name.clone()))
        {
            return Err(YamlFieldError::InvalidMergeSource);
        }
        let inherited = effective_mapping_entries_inner(&source, registry, active_anchors);
        if let Some(name) = anchor {
            let _ = active_anchors.remove(&name);
        }
        for (key, value) in inherited? {
            let key_text = decode_mapping_key(&key, registry)?;
            if seen.insert(key_text) {
                entries.push((key, value));
            }
        }
    }
    Ok(entries)
}

fn merge_source_mappings(
    mapping: &yaml_edit::Mapping,
    registry: &AnchorRegistry,
) -> Result<Vec<(Option<String>, yaml_edit::Mapping)>, YamlFieldError> {
    let mut sources = Vec::new();
    for (key, value) in mapping.iter() {
        if !is_merge_key(&key) {
            continue;
        }
        match value {
            YamlNode::Mapping(source) => push_merge_source(&mut sources, source, None),
            YamlNode::Alias(alias) => push_merge_source(
                &mut sources,
                resolve_alias_mapping(&alias.name(), registry)?,
                Some(alias.name()),
            ),
            YamlNode::Sequence(sequence) => {
                for item in sequence.values() {
                    push_merge_node(&mut sources, item, registry)?;
                }
            }
            YamlNode::Scalar(_) | YamlNode::TaggedNode(_) => {
                return Err(YamlFieldError::InvalidMergeSource);
            }
        }
    }
    Ok(sources)
}

fn push_merge_node(
    sources: &mut Vec<(Option<String>, yaml_edit::Mapping)>,
    node: YamlNode,
    registry: &AnchorRegistry,
) -> Result<(), YamlFieldError> {
    match node {
        YamlNode::Mapping(source) => push_merge_source(sources, source, None),
        YamlNode::Alias(alias) => push_merge_source(
            sources,
            resolve_alias_mapping(&alias.name(), registry)?,
            Some(alias.name()),
        ),
        YamlNode::Scalar(_) | YamlNode::Sequence(_) | YamlNode::TaggedNode(_) => {
            return Err(YamlFieldError::InvalidMergeSource);
        }
    }
    Ok(())
}

fn push_merge_source(
    sources: &mut Vec<(Option<String>, yaml_edit::Mapping)>,
    source: yaml_edit::Mapping,
    anchor: Option<String>,
) {
    sources.push((anchor, source));
}

fn is_merge_key(key: &YamlNode) -> bool {
    key.as_scalar()
        .is_some_and(|scalar| !scalar.is_quoted() && scalar.as_string() == "<<")
}

pub(crate) fn decode_mapping_key(
    key: &YamlNode,
    registry: &AnchorRegistry,
) -> Result<String, YamlFieldError> {
    match key {
        YamlNode::Scalar(scalar) if is_string_scalar(scalar) => Ok(scalar.as_string()),
        YamlNode::TaggedNode(tagged) => decode_tagged_string(tagged),
        YamlNode::Alias(alias) => decode_alias_string(&alias.name(), registry),
        YamlNode::Scalar(_) | YamlNode::Mapping(_) | YamlNode::Sequence(_) => {
            Err(YamlFieldError::WrongShape)
        }
    }
}

fn resolve_alias_mapping(
    name: &str,
    registry: &AnchorRegistry,
) -> Result<yaml_edit::Mapping, YamlFieldError> {
    resolve_alias_document(name, registry)?
        .as_mapping()
        .ok_or(YamlFieldError::InvalidMergeSource)
}

fn validate_merge_alias(
    name: &str,
    registry: &yaml_edit::AnchorRegistry,
) -> Result<(), YamlFieldError> {
    let target = registry
        .resolve(name)
        .ok_or(YamlFieldError::UnresolvedAlias)?;
    Document::from_str(&target.to_string())
        .ok()
        .and_then(|document| document.as_mapping())
        .map(|_| ())
        .ok_or(YamlFieldError::InvalidMergeSource)
}
