//! Git hooks requirement model and merge.

mod merge;
mod model;

pub use model::{
    GitHooksPath, GitHooksRequirements, GitHooksValueError, ResolvedGitHooksRequirements,
};
