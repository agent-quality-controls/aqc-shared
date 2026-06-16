//! Integration tests for the `EngineRequirement` impl on `CargoTomlRequirements`.

use aqc_cargo_toml_engine::CargoTomlRequirements;
use aqc_file_engine_core::EngineRequirement;
use toml_edit as _;

#[test]
fn impl_engine_requirement_id_matches_crate_name() {
    let req = CargoTomlRequirements::default();
    assert_eq!(req.engine_id(), "aqc-cargo-toml-engine");
}

#[test]
fn impl_engine_requirement_downcast_roundtrip() {
    let req: Box<dyn EngineRequirement> = Box::new(CargoTomlRequirements::default());
    let downcast = req.as_any().downcast_ref::<CargoTomlRequirements>();
    assert!(downcast.is_some());
}
