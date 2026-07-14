use std::collections::BTreeMap;

use aqc_file_engine_core::ConfigScalar;
use jsonc_parser::cst::{CstInputValue, CstNode, CstObject, CstRootNode, CstStringLit};

#[derive(Debug)]
pub(crate) struct MaskedNumber {
    pub(crate) literal: CstStringLit,
    pub(crate) marker: String,
    pub(crate) original: String,
}

#[derive(Debug)]
pub struct JsoncObject {
    pub(crate) root: CstRootNode,
    pub(crate) masked_numbers: Vec<MaskedNumber>,
    pub(crate) masked_scalars: BTreeMap<Vec<String>, String>,
    pub(crate) utf8_bom: bool,
}

impl JsoncObject {
    #[must_use]
    pub fn scalar(&self, path: &[&str]) -> Option<ConfigScalar> {
        let value = value_at(&self.root.object_value()?, path)?;
        if let Some(value) = value.as_string_lit() {
            let decoded = value.decoded_value().ok()?;
            if let Some(original) = self.masked_scalars.get(
                &path
                    .iter()
                    .map(|component| (*component).to_owned())
                    .collect::<Vec<_>>(),
            ) {
                return parse_integer(original).map(ConfigScalar::Int);
            }
            return Some(ConfigScalar::Str(decoded));
        }
        if let Some(value) = value.as_boolean_lit() {
            return Some(ConfigScalar::Bool(value.value()));
        }
        value
            .as_number_lit()
            .and_then(|value| parse_integer(&value.to_string()))
            .map(ConfigScalar::Int)
    }

    #[must_use]
    pub fn value_exists(&self, path: &[&str]) -> bool {
        self.root
            .object_value()
            .and_then(|object| value_at(&object, path))
            .is_some()
    }

    #[must_use]
    pub fn object_exists(&self, path: &[&str]) -> bool {
        self.root
            .object_value()
            .and_then(|object| value_at(&object, path))
            .is_some_and(|value| value.as_object().is_some())
    }

    pub fn set_scalar(&mut self, path: &[&str], value: ConfigScalar) -> bool {
        let Some((last, parents)) = path.split_last() else {
            return false;
        };
        let Some(mut object) = self.root.object_value() else {
            return false;
        };
        for parent in parents {
            let Some(next) = object.object_value_or_create(parent) else {
                return false;
            };
            object = next;
        }
        let input = scalar_input(value);
        let _ = self.masked_scalars.remove(
            &path
                .iter()
                .map(|component| (*component).to_owned())
                .collect::<Vec<_>>(),
        );
        if let Some(property) = object.get(last) {
            property.set_value(input);
        } else {
            let _ = object.append(last, input);
        }
        true
    }

    pub fn remove_value(&mut self, path: &[&str]) -> bool {
        let Some((last, parents)) = path.split_last() else {
            return false;
        };
        let Some(mut object) = self.root.object_value() else {
            return false;
        };
        for parent in parents {
            let Some(next) = object.object_value(parent) else {
                return false;
            };
            object = next;
        }
        let Some(property) = object.get(last) else {
            return false;
        };
        let owned_path = path
            .iter()
            .map(|component| (*component).to_owned())
            .collect::<Vec<_>>();
        self.masked_scalars
            .retain(|candidate, _| !candidate.starts_with(&owned_path));
        property.remove();
        true
    }

    #[must_use]
    pub fn rendered_value(&self, path: &[&str]) -> Option<String> {
        if let Some(original) = self.masked_scalars.get(
            &path
                .iter()
                .map(|component| (*component).to_owned())
                .collect::<Vec<_>>(),
        ) {
            return Some(original.clone());
        }
        self.root
            .object_value()
            .and_then(|object| value_at(&object, path))
            .map(|value| value.to_string())
    }

    #[must_use]
    pub fn render(&self) -> Vec<u8> {
        for masked in &self.masked_numbers {
            masked.literal.set_raw_value(masked.original.clone());
        }
        let rendered = self.root.to_string();
        for masked in &self.masked_numbers {
            masked
                .literal
                .set_raw_value(format!("\"{}\"", masked.marker));
        }
        if self.utf8_bom {
            let mut bytes = Vec::with_capacity(rendered.len().saturating_add(3));
            bytes.extend_from_slice(&[0xef, 0xbb, 0xbf]);
            bytes.extend_from_slice(rendered.as_bytes());
            bytes
        } else {
            rendered.into_bytes()
        }
    }
}

fn value_at(object: &CstObject, path: &[&str]) -> Option<CstNode> {
    let (first, rest) = path.split_first()?;
    let mut value = object.get(first)?.value()?;
    for key in rest {
        value = value.as_object()?.get(key)?.value()?;
    }
    Some(value)
}

fn scalar_input(value: ConfigScalar) -> CstInputValue {
    match value {
        ConfigScalar::Str(value) => CstInputValue::String(value),
        ConfigScalar::Int(value) => CstInputValue::Number(value.to_string()),
        ConfigScalar::Bool(value) => CstInputValue::Bool(value),
    }
}

fn parse_integer(value: &str) -> Option<i64> {
    let normalized = value.replace('_', "");
    let normalized_value = normalized.strip_suffix('n').unwrap_or(&normalized);
    let (negative, unsigned) = normalized_value
        .strip_prefix('-')
        .map_or((false, normalized_value), |unsigned| (true, unsigned));
    let parsed = if let Some(hex) = unsigned
        .strip_prefix("0x")
        .or_else(|| unsigned.strip_prefix("0X"))
    {
        i64::from_str_radix(hex, 16).ok()?
    } else if let Some(binary) = unsigned
        .strip_prefix("0b")
        .or_else(|| unsigned.strip_prefix("0B"))
    {
        i64::from_str_radix(binary, 2).ok()?
    } else if let Some(octal) = unsigned
        .strip_prefix("0o")
        .or_else(|| unsigned.strip_prefix("0O"))
    {
        i64::from_str_radix(octal, 8).ok()?
    } else {
        unsigned.parse::<i64>().ok()?
    };
    if negative {
        parsed.checked_neg()
    } else {
        Some(parsed)
    }
}
