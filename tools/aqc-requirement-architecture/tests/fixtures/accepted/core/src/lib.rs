mod requirement {
    pub trait EngineRequirement {}
}

pub use requirement::EngineRequirement;

pub struct KeyedItem<T>(pub T);

pub struct ItemRequirements<T> {
    pub required: Vec<T>,
    pub forbidden: Vec<T>,
    pub exact: Option<Vec<T>>,
}

impl<T> Default for ItemRequirements<T> {
    fn default() -> Self {
        Self {
            required: Vec::new(),
            forbidden: Vec::new(),
            exact: None,
        }
    }
}

impl<T> ItemRequirements<T> {
    pub fn map<U>(self, transform: impl Fn(T) -> U) -> ItemRequirements<U> {
        ItemRequirements {
            required: self.required.into_iter().map(&transform).collect(),
            forbidden: self.forbidden.into_iter().map(&transform).collect(),
            exact: self
                .exact
                .map(|items| items.into_iter().map(transform).collect()),
        }
    }
}
