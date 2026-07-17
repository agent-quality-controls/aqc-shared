use aqc_file_engine_core::{ItemRequirements, KeyedItem};

pub fn policy_constructs_exact() -> ItemRequirements<KeyedItem<()>> {
    ItemRequirements {
        required: Vec::new(),
        forbidden: Vec::new(),
        exact: Some(vec![KeyedItem(())]),
    }
}
