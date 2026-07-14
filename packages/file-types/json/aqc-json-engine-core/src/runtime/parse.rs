use std::fmt;

use aqc_file_engine_core::{Finding, Severity};
use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

use crate::JsonObject;

#[must_use]
#[expect(
    clippy::disallowed_methods,
    reason = "The required duplicate-safe visitor must drive serde_json's deserializer directly before map construction."
)]
pub fn parse_object_or_report(
    current_bytes: Option<&[u8]>,
    file_label: &str,
) -> (Option<JsonObject>, Vec<Finding>) {
    let Some(bytes) = current_bytes else {
        return (
            Some(JsonObject {
                members: Map::new(),
            }),
            Vec::new(),
        );
    };
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    match DuplicateSafeValue
        .deserialize(&mut deserializer)
        .and_then(|value| {
            deserializer.end()?;
            match value {
                Value::Object(members) => Ok(JsonObject { members }),
                Value::Null
                | Value::Bool(_)
                | Value::Number(_)
                | Value::String(_)
                | Value::Array(_) => Err(serde_json::Error::custom("root value must be an object")),
            }
        }) {
        Ok(object) => (Some(object), Vec::new()),
        Err(error) => (
            None,
            vec![Finding::ParseError {
                message: format!("{file_label} is not a valid JSON object: {error}"),
                severity: Severity::Error,
            }],
        ),
    }
}

struct DuplicateSafeValue;

impl<'de> DeserializeSeed<'de> for DuplicateSafeValue {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a duplicate-free JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(DuplicateSafeValue)? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut mapping: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut members = Map::new();
        while let Some(key) = mapping.next_key::<String>()? {
            if members.contains_key(&key) {
                return Err(A::Error::custom(format!("duplicate object member `{key}`")));
            }
            let value = mapping.next_value_seed(DuplicateSafeValue)?;
            let _ = members.insert(key, value);
        }
        Ok(Value::Object(members))
    }
}
