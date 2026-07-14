//! Direct root-mapping mutation.

use std::collections::BTreeMap;

use std::str::FromStr;

use yaml_edit::{Document, ScalarValue, SequenceBuilder};

pub(crate) fn set_string(document: &Document, key: &str, value: &str) {
    if let Some(mapping) = document.as_mapping() {
        mapping.set(key, value);
    }
}

pub(crate) fn set_boolean(document: &Document, key: &str, value: bool) {
    if let Some(mapping) = document.as_mapping() {
        mapping.set(key, value);
    }
}

pub(crate) fn set_integer(document: &Document, key: &str, value: u64) {
    if let Some(mapping) = document.as_mapping() {
        mapping.set(key, value);
    }
}

pub(crate) fn set_string_sequence(document: &Document, key: &str, values: &[String]) {
    if values.is_empty() {
        let empty_sequence = Document::from_str("[]")
            .ok()
            .and_then(|generated| generated.as_sequence());
        if let (Some(mapping), Some(empty_sequence)) = (document.as_mapping(), empty_sequence) {
            let _ = mapping.remove(key);
            mapping.set(key, empty_sequence);
        }
        return;
    }
    let mut builder = SequenceBuilder::new();
    for value in values {
        builder = builder.item(value.as_str());
    }
    if let (Some(mapping), Some(sequence)) = (
        document.as_mapping(),
        builder.build_document().as_sequence(),
    ) {
        let _ = mapping.remove(key);
        mapping.set(key, sequence);
    }
}

pub(crate) fn set_string_boolean_mapping(
    document: &Document,
    key: &str,
    values: &BTreeMap<String, bool>,
) {
    let mut yaml = String::from("value: {");
    for (index, (item_key, value)) in values.iter().enumerate() {
        if index > 0 {
            yaml.push_str(", ");
        }
        yaml.push_str(&ScalarValue::double_quoted(item_key).to_yaml_string());
        yaml.push_str(": ");
        yaml.push_str(if *value { "true" } else { "false" });
    }
    yaml.push_str("}\n");
    let value_mapping = Document::from_str(&yaml)
        .ok()
        .and_then(|generated| generated.as_mapping())
        .and_then(|generated| generated.get_mapping("value"));
    if let (Some(mapping), Some(value_mapping)) = (document.as_mapping(), value_mapping) {
        let _ = mapping.remove(key);
        mapping.set(key, value_mapping);
    }
}

pub(crate) fn remove(document: &Document, key: &str) -> bool {
    document
        .as_mapping()
        .and_then(|mapping| mapping.remove(key))
        .is_some()
}
