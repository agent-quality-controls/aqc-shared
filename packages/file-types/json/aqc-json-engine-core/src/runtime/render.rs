use crate::JsonObject;

#[must_use]
pub fn render_object(object: &JsonObject) -> Vec<u8> {
    let mut rendered =
        serde_json::to_vec_pretty(&object.members).unwrap_or_else(|_| b"{}".to_vec());
    rendered.push(b'\n');
    rendered
}
