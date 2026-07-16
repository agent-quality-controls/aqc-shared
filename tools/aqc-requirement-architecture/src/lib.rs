mod analyze;
mod discover;
mod expression;
mod fs;
mod model;

pub use analyze::check_repository_roots;
pub use model::{ArchitectureError, ArchitectureReport, RequirementKind, ViolationCode};
