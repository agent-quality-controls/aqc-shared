//! `CargoTomlEngine`: the engine struct and its `FileEngine` impl.

use aqc_file_engine_core::{EngineError, EngineOutput, FileEngine};
use toml_edit::DocumentMut;

use crate::reconcile::apply_requirement;
use crate::requirement::CargoTomlRequirement;

/// `Cargo.toml` engine.
#[derive(Debug, Default)]
pub struct CargoTomlEngine;

impl FileEngine<CargoTomlRequirement> for CargoTomlEngine {
    fn reconcile(
        current_bytes: Option<&[u8]>,
        requirement: &CargoTomlRequirement,
    ) -> Result<EngineOutput, EngineError> {
        let mut doc = parse_document(current_bytes)?;
        let mut findings = Vec::new();
        apply_requirement(&mut doc, requirement, &mut findings);
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
