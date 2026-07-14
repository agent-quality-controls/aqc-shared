//! pnpm workspace merge dispatch and byte reconciliation.

mod engine;
mod reconcile;
mod scalar;
mod selector;

pub use engine::PnpmWorkspaceYamlEngine;
pub(crate) use selector::{compile_selector_glob, selector_matches};
