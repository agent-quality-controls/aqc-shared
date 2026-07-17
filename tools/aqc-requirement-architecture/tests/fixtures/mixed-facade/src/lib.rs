pub use aqc_file_engine_core::{ItemRequirements, KeyedItem};

pub mod counterfeit {
    pub struct ItemRequirements<T>(pub T);
    pub struct KeyedItem<T>(pub T);
}
