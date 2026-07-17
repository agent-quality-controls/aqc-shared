use aqc_file_engine_core::{ItemRequirements, KeyedItem};

pub fn policy_constructs_exact() -> ItemRequirements<KeyedItem<()>> {
    ItemRequirements {
        required: Vec::new(),
        forbidden: Vec::new(),
        allowed: None,
        exact: Some(vec![KeyedItem(())]),
    }
}

pub fn policy_constructs_allowed() -> ItemRequirements<KeyedItem<()>> {
    ItemRequirements {
        required: Vec::new(),
        forbidden: Vec::new(),
        allowed: Some(vec![KeyedItem(())]),
        exact: None,
    }
}
