use aqc_file_engine_core::ConfigScalar;
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonObject {
    pub(crate) members: Map<String, Value>,
}

impl JsonObject {
    #[must_use]
    pub fn scalar(&self, path: &[&str]) -> Option<ConfigScalar> {
        value_at(&self.members, path).and_then(value_to_scalar)
    }

    #[must_use]
    pub fn value_exists(&self, path: &[&str]) -> bool {
        value_at(&self.members, path).is_some()
    }

    #[must_use]
    pub fn object_exists(&self, path: &[&str]) -> bool {
        value_at(&self.members, path).is_some_and(Value::is_object)
    }

    pub fn set_scalar(&mut self, path: &[&str], value: ConfigScalar) -> bool {
        set_value(&mut self.members, path, scalar_to_value(value))
    }

    pub fn remove_value(&mut self, path: &[&str]) -> bool {
        remove_value(&mut self.members, path)
    }

    #[must_use]
    pub fn rendered_value(&self, path: &[&str]) -> Option<String> {
        value_at(&self.members, path).map(Value::to_string)
    }
}

fn value_at<'a>(members: &'a Map<String, Value>, path: &[&str]) -> Option<&'a Value> {
    let (first, rest) = path.split_first()?;
    let mut value = members.get(*first)?;
    for key in rest {
        value = value.as_object()?.get(*key)?;
    }
    Some(value)
}

fn value_to_scalar(value: &Value) -> Option<ConfigScalar> {
    match value {
        Value::String(value) => Some(ConfigScalar::Str(value.clone())),
        Value::Bool(value) => Some(ConfigScalar::Bool(*value)),
        Value::Number(value) => value.as_i64().map(ConfigScalar::Int),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

fn scalar_to_value(value: ConfigScalar) -> Value {
    match value {
        ConfigScalar::Str(value) => Value::String(value),
        ConfigScalar::Int(value) => Value::Number(value.into()),
        ConfigScalar::Bool(value) => Value::Bool(value),
    }
}

fn set_value(members: &mut Map<String, Value>, path: &[&str], value: Value) -> bool {
    let Some((last, parents)) = path.split_last() else {
        return false;
    };
    let mut current = members;
    for key in parents {
        let entry = current
            .entry((*key).to_owned())
            .or_insert_with(|| Value::Object(Map::new()));
        if !entry.is_object() {
            *entry = Value::Object(Map::new());
        }
        let Some(object) = entry.as_object_mut() else {
            return false;
        };
        current = object;
    }
    let _ = current.insert((*last).to_owned(), value);
    true
}

fn remove_value(members: &mut Map<String, Value>, path: &[&str]) -> bool {
    let Some((last, parents)) = path.split_last() else {
        return false;
    };
    let mut current = members;
    for key in parents {
        let Some(object) = current.get_mut(*key).and_then(Value::as_object_mut) else {
            return false;
        };
        current = object;
    }
    current.remove(*last).is_some()
}
