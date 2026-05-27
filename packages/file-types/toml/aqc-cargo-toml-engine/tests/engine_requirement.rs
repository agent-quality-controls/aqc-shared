//! Integration tests for the `EngineRequirement` impl on `CargoTomlRequirement`.

use aqc_cargo_toml_engine::CargoTomlRequirement;
use aqc_file_engine_core::EngineRequirement;
use toml_edit as _;

#[test]
fn impl_engine_requirement_id_matches_crate_name() {
    let req = CargoTomlRequirement::default();
    assert_eq!(req.engine_id(), "aqc-cargo-toml-engine");
}

#[test]
fn impl_engine_requirement_downcast_roundtrip() {
    let req: Box<dyn EngineRequirement> = Box::new(CargoTomlRequirement::default());
    let downcast = req.as_any().downcast_ref::<CargoTomlRequirement>();
    assert!(downcast.is_some());
}
