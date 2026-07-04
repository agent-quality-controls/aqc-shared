//! `RustToolchainTomlEngine`: engine struct and `FileEngine` + `Engine` impls.

use std::path::{Path, PathBuf};

use aqc_file_engine_core::{
    Engine, EngineOutput, EngineRequirement, FileEngine, Provenance, merged_reconcile,
};
use aqc_toml_engine_core::parse_or_report;

use crate::reconcile;
use crate::requirement::{ResolvedRustToolchainTomlRequirements, RustToolchainTomlRequirements};

#[derive(Debug, Default)]
pub struct RustToolchainTomlEngine;

impl FileEngine<ResolvedRustToolchainTomlRequirements> for RustToolchainTomlEngine {
    fn reconcile(
        current_bytes: Option<&[u8]>,
        requirement: &ResolvedRustToolchainTomlRequirements,
    ) -> EngineOutput {
        let (mut doc, mut findings) = parse_or_report(current_bytes, "rust-toolchain.toml");
        if !findings.is_empty() {
            return EngineOutput {
                expected_bytes: Vec::new(),
                findings,
            };
        }
        reconcile::apply(&mut doc, requirement, &mut findings);
        EngineOutput {
            expected_bytes: doc.to_string().into_bytes(),
            findings,
        }
    }
}

impl Engine for RustToolchainTomlEngine {
    fn id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn target_path(&self, workspace_root: &Path) -> PathBuf {
        workspace_root.join("rust-toolchain.toml")
    }

    fn reconcile(
        &self,
        current: Option<&[u8]>,
        reqs: &[(Provenance, Box<dyn EngineRequirement>)],
    ) -> EngineOutput {
        merged_reconcile(
            current,
            reqs,
            "rust-toolchain.toml",
            RustToolchainTomlRequirements::merge,
            <Self as FileEngine<ResolvedRustToolchainTomlRequirements>>::reconcile,
        )
    }
}
