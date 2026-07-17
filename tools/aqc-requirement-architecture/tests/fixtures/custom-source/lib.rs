pub trait EngineRequirement {}
use EngineRequirement as Contract;

pub struct RootRequirement {
    pub enabled: bool,
}
impl EngineRequirement for RootRequirement {}

#[path = "requirements.rs"]
mod requirements;
mod nested_alias;

mod outer {
    #[path = "hidden.rs"]
    mod inner;
}
