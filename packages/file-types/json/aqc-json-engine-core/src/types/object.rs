use std::collections::BTreeMap;

use aqc_file_engine_core::ConfigScalar;
use jsonc_parser::cst::{CstInputValue, CstNode, CstObject, CstRootNode, CstStringLit};

#[derive(Debug)]
pub(crate) struct MaskedNumber {
    pub(crate) literal: CstStringLit,
    pub(crate) marker: String,
    pub(crate) original: String,
    pub(crate) path: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct MaskedString {
    pub(crate) literal: CstStringLit,
    pub(crate) masked: String,
    pub(crate) original: String,
    pub(crate) unrepresentable: bool,
    pub(crate) path: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonObjectParentAction {
    Preserve,
    Replace,
}

#[derive(Debug)]
pub struct JsonObject {
    pub(crate) root: CstRootNode,
    pub(crate) masked_numbers: Vec<MaskedNumber>,
    pub(crate) masked_strings: Vec<MaskedString>,
    pub(crate) masked_scalars: BTreeMap<Vec<String>, String>,
    pub(crate) utf8_bom: bool,
}

impl JsonObject {
    #[must_use]
    pub fn scalar(&self, path: &[&str]) -> Option<ConfigScalar> {
        let value = value_at(&self.root.object_value()?, path)?;
        if let Some(value) = value.as_string_lit() {
            let owned_path = path
                .iter()
                .map(|component| (*component).to_owned())
                .collect::<Vec<_>>();
            if self
                .masked_strings
                .iter()
                .any(|masked| masked.unrepresentable && masked.path.as_ref() == Some(&owned_path))
            {
                return None;
            }
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
        if path.is_empty() {
            return self.root.object_value().is_some();
        }
        self.root
            .object_value()
            .and_then(|object| value_at(&object, path))
            .is_some_and(|value| value.as_object().is_some())
    }

    #[must_use]
    pub fn object_keys(&self, path: &[&str]) -> Option<Vec<String>> {
        object_at(&self.root, path)?
            .properties()
            .into_iter()
            .map(|property| property.name()?.decoded_value().ok())
            .collect()
    }

    pub fn remove_object_key(&mut self, path: &[&str], key: &str) -> bool {
        let Some(object) = object_at(&self.root, path) else {
            return false;
        };
        let Some(property) = object.get(key) else {
            return false;
        };
        property.remove();
        let mut metadata_path = path.to_vec();
        metadata_path.push(key);
        self.discard_path_metadata(&metadata_path);
        true
    }

    pub fn set_scalar(
        &mut self,
        path: &[&str],
        value: ConfigScalar,
        parent_action: NonObjectParentAction,
    ) -> bool {
        let Some((last, parents)) = path.split_last() else {
            return false;
        };
        let Some((object, replaced_parent_depth)) =
            object_for_write(&self.root, parents, parent_action)
        else {
            return false;
        };
        let input = scalar_input(value);
        if let Some(property) = object.get(last) {
            property.set_value(input);
        } else {
            let _ = object.append(last, input);
        }
        let metadata_path = replaced_parent_depth
            .and_then(|depth| path.get(..depth))
            .unwrap_or(path);
        self.discard_path_metadata(metadata_path);
        true
    }

    #[must_use]
    pub fn string_list(&self, path: &[&str]) -> Option<Vec<String>> {
        value_at(&self.root.object_value()?, path)?
            .as_array()?
            .elements()
            .into_iter()
            .map(|node| node.as_string_lit()?.decoded_value().ok())
            .collect()
    }

    #[must_use]
    pub fn value_is_array(&self, path: &[&str]) -> bool {
        self.root
            .object_value()
            .and_then(|object| value_at(&object, path))
            .is_some_and(|value| value.as_array().is_some())
    }

    pub fn set_string_list(
        &mut self,
        path: &[&str],
        values: &[String],
        parent_action: NonObjectParentAction,
    ) -> bool {
        let Some((last, parents)) = path.split_last() else {
            return false;
        };
        let Some((object, replaced_parent_depth)) =
            object_for_write(&self.root, parents, parent_action)
        else {
            return false;
        };
        let input =
            CstInputValue::Array(values.iter().cloned().map(CstInputValue::String).collect());
        if let Some(property) = object.get(last) {
            property.set_value(input);
        } else {
            let _ = object.append(last, input);
        }
        let metadata_path = replaced_parent_depth
            .and_then(|depth| path.get(..depth))
            .unwrap_or(path);
        self.discard_path_metadata(metadata_path);
        true
    }

    pub fn set_object(&mut self, path: &[&str], parent_action: NonObjectParentAction) -> bool {
        if path.is_empty() {
            return self.root.object_value().is_some();
        }
        let Some((last, parents)) = path.split_last() else {
            return false;
        };
        let Some((object, replaced_parent_depth)) =
            object_for_write(&self.root, parents, parent_action)
        else {
            return false;
        };
        if object.object_value(last).is_some() {
            return true;
        }
        if object.get(last).is_some() && parent_action == NonObjectParentAction::Preserve {
            return false;
        }
        if let Some(property) = object.get(last) {
            property.set_value(CstInputValue::Object(Vec::new()));
        } else {
            let _ = object.append(last, CstInputValue::Object(Vec::new()));
        }
        let metadata_path = replaced_parent_depth
            .and_then(|depth| path.get(..depth))
            .unwrap_or(path);
        self.discard_path_metadata(metadata_path);
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
        property.remove();
        self.discard_path_metadata(path);
        true
    }

    #[must_use]
    pub fn rendered_value(&self, path: &[&str]) -> Option<String> {
        let value = self
            .root
            .object_value()
            .and_then(|object| value_at(&object, path))?;
        if let Some(original) = self.masked_scalars.get(
            &path
                .iter()
                .map(|component| (*component).to_owned())
                .collect::<Vec<_>>(),
        ) {
            return Some(original.clone());
        }
        Some(self.render_with_original_scalars(|| value.to_string()))
    }

    #[must_use]
    pub fn render(&self) -> Vec<u8> {
        let rendered = self.render_with_original_scalars(|| self.root.to_string());
        if self.utf8_bom {
            let mut bytes = Vec::with_capacity(rendered.len().saturating_add(3));
            bytes.extend_from_slice(&[0xef, 0xbb, 0xbf]);
            bytes.extend_from_slice(rendered.as_bytes());
            bytes
        } else {
            rendered.into_bytes()
        }
    }

    fn render_with_original_scalars(&self, render: impl FnOnce() -> String) -> String {
        for masked in &self.masked_numbers {
            masked.literal.set_raw_value(masked.original.clone());
        }
        for masked in &self.masked_strings {
            masked.literal.set_raw_value(masked.original.clone());
        }
        let rendered = render();
        for masked in &self.masked_numbers {
            masked
                .literal
                .set_raw_value(format!("\"{}\"", masked.marker));
        }
        for masked in &self.masked_strings {
            masked.literal.set_raw_value(masked.masked.clone());
        }
        rendered
    }

    fn discard_path_metadata(&mut self, path: &[&str]) {
        let owned_path = path
            .iter()
            .map(|component| (*component).to_owned())
            .collect::<Vec<_>>();
        self.masked_scalars
            .retain(|candidate, _| !candidate.starts_with(&owned_path));
        self.masked_numbers
            .retain(|masked| !masked.path.starts_with(&owned_path));
        self.masked_strings.retain(|masked| {
            masked
                .path
                .as_ref()
                .is_none_or(|candidate| !candidate.starts_with(&owned_path))
        });
    }
}

fn object_for_write(
    root: &CstRootNode,
    parents: &[&str],
    parent_action: NonObjectParentAction,
) -> Option<(CstObject, Option<usize>)> {
    let mut object = root.object_value()?;
    let mut replaced_parent_depth = None;
    for (index, parent) in parents.iter().enumerate() {
        let replaces_existing_value = parent_action == NonObjectParentAction::Replace
            && object.get(parent).is_some()
            && object.object_value(parent).is_none();
        let next = match parent_action {
            NonObjectParentAction::Preserve => object.object_value_or_create(parent),
            NonObjectParentAction::Replace => Some(object.object_value_or_set(parent)),
        }?;
        if replaces_existing_value && replaced_parent_depth.is_none() {
            replaced_parent_depth = Some(index.saturating_add(1));
        }
        object = next;
    }
    Some((object, replaced_parent_depth))
}

fn value_at(object: &CstObject, path: &[&str]) -> Option<CstNode> {
    let (first, rest) = path.split_first()?;
    let mut value = object.get(first)?.value()?;
    for key in rest {
        value = value.as_object()?.get(key)?.value()?;
    }
    Some(value)
}

fn object_at(root: &CstRootNode, path: &[&str]) -> Option<CstObject> {
    let root_object = root.object_value()?;
    if path.is_empty() {
        Some(root_object)
    } else {
        value_at(&root_object, path)?.as_object()
    }
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
    let (negative, unsigned) = normalized_value.strip_prefix('-').map_or_else(
        || {
            (
                false,
                normalized_value
                    .strip_prefix('+')
                    .unwrap_or(normalized_value),
            )
        },
        |unsigned| (true, unsigned),
    );
    let parsed = if let Some(hex) = unsigned
        .strip_prefix("0x")
        .or_else(|| unsigned.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).ok()?
    } else if let Some(binary) = unsigned
        .strip_prefix("0b")
        .or_else(|| unsigned.strip_prefix("0B"))
    {
        u64::from_str_radix(binary, 2).ok()?
    } else if let Some(octal) = unsigned
        .strip_prefix("0o")
        .or_else(|| unsigned.strip_prefix("0O"))
    {
        u64::from_str_radix(octal, 8).ok()?
    } else {
        unsigned.parse::<u64>().ok()?
    };
    if negative {
        i64::try_from(parsed)
            .ok()
            .and_then(i64::checked_neg)
            .or_else(|| (parsed == i64::MIN.unsigned_abs()).then_some(i64::MIN))
    } else {
        i64::try_from(parsed).ok()
    }
}
