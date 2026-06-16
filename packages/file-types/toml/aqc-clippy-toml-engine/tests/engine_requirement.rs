//! Integration tests for the `EngineRequirement` impl on `ClippyTomlRequirements`.

use aqc_clippy_toml_engine::ClippyTomlRequirements;
use aqc_file_engine_core::EngineRequirement;
use toml_edit as _;

#[test]
fn impl_engine_requirement_id_matches_crate_name() {
    let req = ClippyTomlRequirements::default();
    assert_eq!(req.engine_id(), "aqc-clippy-toml-engine");
}

#[test]
fn impl_engine_requirement_downcast_roundtrip() {
    let req: Box<dyn EngineRequirement> = Box::new(ClippyTomlRequirements::default());
    let downcast = req.as_any().downcast_ref::<ClippyTomlRequirements>();
    assert!(downcast.is_some());
}
