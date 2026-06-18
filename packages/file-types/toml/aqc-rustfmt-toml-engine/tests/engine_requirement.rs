use aqc_file_engine_core::EngineRequirement;
use aqc_rustfmt_toml_engine::{ENGINE_ID, RustfmtTomlRequirements};
use globset as _;
use toml_edit as _;

#[test]
fn requirement_reports_rustfmt_engine_id() {
    let req = RustfmtTomlRequirements::default();

    assert_eq!(
        req.engine_id(),
        ENGINE_ID,
        "engine id must route to rustfmt"
    );
}
