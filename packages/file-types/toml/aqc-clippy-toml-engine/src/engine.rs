//! `ClippyTomlEngine`: the engine struct and its `FileEngine` impl.

use aqc_file_engine_core::{EngineOutput, FileEngine, parse_or_report};

use crate::reconcile;
use crate::requirement::ClippyTomlRequirement;

/// `clippy.toml` engine.
#[derive(Debug, Default)]
pub struct ClippyTomlEngine;

impl FileEngine<ClippyTomlRequirement> for ClippyTomlEngine {
    fn reconcile(
        current_bytes: Option<&[u8]>,
        requirement: &ClippyTomlRequirement,
    ) -> EngineOutput {
        let (mut doc, mut findings) = parse_or_report(current_bytes, "clippy.toml");
        reconcile::apply(&mut doc, requirement, &mut findings);
        EngineOutput {
            expected_bytes: doc.to_string().into_bytes(),
            findings,
        }
    }
}
