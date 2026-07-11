use aqc_file_engine_core::EngineRequirement;
use aqc_rust_toolchain_toml_engine::{
    ENGINE_ID, RustToolchainChannel, RustToolchainProfile, RustToolchainTomlRequirements,
};
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
use schemars as _;

#[test]
fn public_toolchain_value_schemas_match_their_wire_values() {
    let channel = schemars::schema_for!(RustToolchainChannel);
    assert_eq!(
        channel.get("type").and_then(|value| value.as_str()),
        Some("string")
    );
    assert_eq!(
        channel.get("minLength").and_then(|value| value.as_u64()),
        Some(1)
    );

    let profile = schemars::schema_for!(RustToolchainProfile);
    assert_eq!(
        profile
            .get("enum")
            .and_then(|value| value.as_array())
            .map(|values| values
                .iter()
                .filter_map(|value| value.as_str())
                .collect::<Vec<_>>()),
        Some(vec!["minimal", "default", "complete"])
    );
}
