//! Lossless parsed YAML mapping.

use std::collections::BTreeMap;

use core::cell::Cell;

use yaml_edit::Document;

use crate::runtime;
use crate::types::{YamlFieldError, YamlFieldValue};

#[derive(Debug, Clone)]
pub struct ParsedYamlMapping {
    pub(crate) document: Document,
    pub(crate) original: Option<Vec<u8>>,
    pub(crate) dirty: Cell<bool>,
}

impl ParsedYamlMapping {
    #[must_use]
    pub fn direct_keys(&self) -> Vec<String> {
        runtime::root_keys(&self.document)
    }

    /// Returns root setting names visible after YAML merge-key resolution.
    ///
    /// # Errors
    ///
    /// Returns a field error when a merge source or effective key has the wrong shape.
    pub fn effective_keys(&self) -> Result<Vec<String>, YamlFieldError> {
        runtime::effective_root_keys(&self.document)
    }

    /// Reads one effective field through YAML merge and alias semantics.
    ///
    /// # Errors
    ///
    /// Returns a field error for unresolved aliases, invalid merge sources,
    /// unknown tags, or a value that cannot be represented by the format API.
    pub fn field(&self, key: &str) -> Result<Option<YamlFieldValue>, YamlFieldError> {
        runtime::read_field(&self.document, key)
    }

    pub fn set_string(&self, key: &str, value: &str) {
        runtime::set_string(&self.document, key, value);
        self.dirty.set(true);
    }

    pub fn set_boolean(&self, key: &str, value: bool) {
        runtime::set_boolean(&self.document, key, value);
        self.dirty.set(true);
    }

    pub fn set_integer(&self, key: &str, value: u64) {
        runtime::set_integer(&self.document, key, value);
        self.dirty.set(true);
    }

    pub fn set_string_sequence(&self, key: &str, values: &[String]) {
        runtime::set_string_sequence(&self.document, key, values);
        self.dirty.set(true);
    }

    pub fn set_string_boolean_mapping(&self, key: &str, values: &BTreeMap<String, bool>) {
        runtime::set_string_boolean_mapping(&self.document, key, values);
        self.dirty.set(true);
    }

    pub fn remove(&self, key: &str) {
        if runtime::remove(&self.document, key) {
            self.dirty.set(true);
        }
    }

    /// Removes a direct key only when merge resolution will not expose it again.
    #[must_use]
    pub fn remove_if_effectively_absent(&self, key: &str) -> bool {
        let candidate = self.clone();
        candidate.remove(key);
        if !matches!(candidate.field(key), Ok(None)) {
            return false;
        }
        self.remove(key);
        true
    }

    #[must_use]
    pub fn render(&self) -> Vec<u8> {
        if !self.dirty.get() {
            if let Some(original) = &self.original {
                return original.clone();
            }
        }
        let rendered = self.document.to_string();
        if rendered.is_empty() {
            b"{}\n".to_vec()
        } else {
            rendered.into_bytes()
        }
    }
}
