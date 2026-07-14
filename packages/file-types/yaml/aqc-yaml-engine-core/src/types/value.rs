//! Decoded YAML field values and field-level shape failures.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YamlFieldValue {
    String(String),
    Boolean(bool),
    Integer(u64),
    StringSequence(Vec<String>),
    StringBooleanMapping(BTreeMap<String, bool>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YamlFieldError {
    WrongShape,
    UnresolvedAlias,
    InvalidMergeSource,
    UnknownTag,
}
