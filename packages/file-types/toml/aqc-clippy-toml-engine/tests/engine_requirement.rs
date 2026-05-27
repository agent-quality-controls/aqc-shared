//! Integration tests for the `EngineRequirement` impl on `ClippyTomlRequirement`.

use aqc_clippy_toml_engine::ClippyTomlRequirement;
use aqc_file_engine_core::EngineRequirement;
use toml_edit as _;

#[test]
fn impl_engine_requirement_id_matches_crate_name() {
    let req = ClippyTomlRequirement::default();
    assert_eq!(req.engine_id(), "aqc-clippy-toml-engine");
}

#[test]
fn impl_engine_requirement_downcast_roundtrip() {
    let req: Box<dyn EngineRequirement> = Box::new(ClippyTomlRequirement::default());
    let downcast = req.as_any().downcast_ref::<ClippyTomlRequirement>();
    assert!(downcast.is_some());
}
