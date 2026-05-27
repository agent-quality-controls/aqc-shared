//! `ClippyTomlEngine`: the engine struct and its `FileEngine` impl.

use aqc_file_engine_core::{EngineError, EngineOutput, FileEngine};
use toml_edit::DocumentMut;

use crate::reconcile::{apply_method_bans, apply_msrv, apply_thresholds};
use crate::requirement::ClippyTomlRequirement;

/// `clippy.toml` engine.
#[derive(Debug, Default)]
pub struct ClippyTomlEngine;

impl FileEngine<ClippyTomlRequirement> for ClippyTomlEngine {
    fn reconcile(
        current_bytes: Option<&[u8]>,
        requirement: &ClippyTomlRequirement,
    ) -> Result<EngineOutput, EngineError> {
        let mut doc = parse_document(current_bytes)?;
        let mut findings = Vec::new();

        if let Some(msrv) = &requirement.msrv {
            apply_msrv(&mut doc, msrv, &mut findings);
        }
        if let Some(thresholds) = &requirement.thresholds {
            apply_thresholds(&mut doc, thresholds, &mut findings);
        }
        if let Some(bans) = &requirement.method_bans {
            apply_method_bans(&mut doc, bans, &mut findings);
        }

        Ok(EngineOutput {
            expected_bytes: doc.to_string().into_bytes(),
            findings,
        })
    }
}

/// Parse the current file contents (or an empty document if `None`).
fn parse_document(current_bytes: Option<&[u8]>) -> Result<DocumentMut, EngineError> {
    let text = match current_bytes {
        Some(bytes) => std::str::from_utf8(bytes).map_err(|e| EngineError::Parse(e.to_string()))?,
        None => "",
    };
    text.parse::<DocumentMut>()
        .map_err(|e| EngineError::Parse(e.to_string()))
}
