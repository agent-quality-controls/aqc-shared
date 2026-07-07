//! Shared deny.toml reconciliation helpers.

use aqc_toml_engine_core::{ensure_nested, ensure_table, ensure_table_at, table_ref};
use toml_edit::{Array, DocumentMut, Item, Table, TableLike, Value};

pub(super) fn table_item<'a>(
    doc: &'a DocumentMut,
    table_path: &[&str],
    field_key: &str,
) -> Option<&'a Item> {
    if table_path.is_empty() {
        return doc.get(field_key);
    }
    let path = table_path
        .iter()
        .map(|segment| (*segment).to_owned())
        .collect::<Vec<_>>();
    table_ref_at(doc, &path).and_then(|table| table.get(field_key))
}

fn table_ref_at<'a>(doc: &'a DocumentMut, path: &[String]) -> Option<&'a dyn TableLike> {
    let _ = table_ref(doc, path.first()?.as_str());
    let (first, rest) = path.split_first()?;
    let mut cur = doc.get(first)?.as_table_like()?;
    for segment in rest {
        cur = cur.get(segment)?.as_table_like()?;
    }
    Some(cur)
}

pub(super) fn table_path_mut<'a>(
    doc: &'a mut DocumentMut,
    table_path: &[&str],
) -> Option<&'a mut Table> {
    let path = table_path
        .iter()
        .map(|segment| (*segment).to_owned())
        .collect::<Vec<_>>();
    aqc_toml_engine_core::table_at_mut(doc, &path)
}

pub(super) fn ensure_table_path<'a>(
    doc: &'a mut DocumentMut,
    table_path: &[&str],
) -> &'a mut Table {
    if let Some((first, rest)) = table_path.split_first() {
        let mut table = ensure_table(doc, first);
        for segment in rest {
            table = ensure_nested(table, segment);
        }
        table
    } else {
        ensure_table_at(doc, &["__root__".to_owned()])
    }
}

pub(super) fn string_array_item(values: &[String]) -> Item {
    let mut array = Array::new();
    for value in values {
        array.push(Value::from(value.as_str()));
    }
    Item::Value(Value::Array(array))
}

pub(super) fn render_item(item: &Item) -> String {
    item.to_string().trim().to_owned()
}
