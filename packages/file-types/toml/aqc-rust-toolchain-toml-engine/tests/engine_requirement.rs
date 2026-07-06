use aqc_file_engine_core::EngineRequirement;
use aqc_rust_toolchain_toml_engine::{ENGINE_ID, RustToolchainTomlRequirements};
use aqc_toml_engine_core as _;
use serde as _;
use toml_edit as _;

#[test]
fn requirement_reports_rust_toolchain_engine_id() {
    let req = RustToolchainTomlRequirements::default();

    assert_eq!(
        req.engine_id(),
        ENGINE_ID,
        "engine id must route to rust-toolchain.toml"
    );
}
