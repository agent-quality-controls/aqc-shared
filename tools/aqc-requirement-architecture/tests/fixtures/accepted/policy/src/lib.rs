pub struct KeyedItem<T>(T);

pub struct ItemRequirements<T> {
    pub required: Vec<T>,
    pub forbidden: Vec<T>,
    pub exact: Option<Vec<T>>,
}

pub fn policy_constructs_exact() -> ItemRequirements<KeyedItem<()>> {
    ItemRequirements {
        required: Vec::new(),
        forbidden: Vec::new(),
        exact: Some(vec![KeyedItem(())]),
    }
}
