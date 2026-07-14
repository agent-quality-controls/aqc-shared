use std::collections::{BTreeMap, BTreeSet};

use jsonc_parser::cst::{CstNode, CstObject, CstRootNode};

use crate::types::{MaskedNumber, MaskedString};

use super::extensions::MaskedStringSource;

pub(super) fn bind_masked_numbers(
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

pub(super) fn bind_masked_strings(
    root: &CstRootNode,
    sources: &[MaskedStringSource],
    number_markers: &BTreeSet<String>,
) -> Vec<MaskedString> {
    let mut literals = Vec::new();
    if let Some(object) = root.object_value() {
        collect_object_strings(&object, &mut Vec::new(), true, &mut literals);
    }
    literals
        .into_iter()
        .filter(|collected| {
            !number_markers
                .iter()
                .any(|marker| collected.literal.raw_value() == format!("\"{marker}\""))
        })
        .zip(sources)
        .filter_map(|(collected, source)| {
            (collected.literal.raw_value() == source.masked).then(|| MaskedString {
                literal: collected.literal,
                masked: source.masked.clone(),
                original: source.original.clone(),
                unrepresentable: source.unrepresentable,
                path: collected.path,
            })
        })
        .collect()
}

struct CollectedString {
    literal: jsonc_parser::cst::CstStringLit,
    path: Option<Vec<String>>,
}

fn collect_object_strings(
    object: &CstObject,
    path: &mut Vec<String>,
    addressable: bool,
    strings: &mut Vec<CollectedString>,
) {
    for property in object.properties() {
        if let Some(name) = property.name().and_then(|name| name.as_string_lit()) {
            strings.push(CollectedString {
                literal: name,
                path: None,
            });
        }
        if let Some(value) = property.value()
            && let Some(name) = property.name().and_then(|name| name.decoded_value().ok())
        {
            path.push(name);
            collect_node_strings(&value, path, addressable, strings);
            let _ = path.pop();
        }
    }
}

fn collect_node_strings(
    node: &CstNode,
    path: &mut Vec<String>,
    addressable: bool,
    strings: &mut Vec<CollectedString>,
) {
    if let Some(literal) = node.as_string_lit() {
        strings.push(CollectedString {
            literal,
            path: addressable.then(|| path.clone()),
        });
    } else if let Some(object) = node.as_object() {
        collect_object_strings(&object, path, addressable, strings);
    } else if let Some(array) = node.as_array() {
        for element in array.elements() {
            collect_node_strings(&element, path, false, strings);
        }
    }
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
        && literal.to_string() == format!("\"{marker}\"")
    {
        masked_numbers.push(MaskedNumber {
            literal,
            marker,
            original: original.clone(),
            path: path.clone(),
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
