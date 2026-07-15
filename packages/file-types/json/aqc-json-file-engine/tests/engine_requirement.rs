use aqc_json_engine_core as _;
use aqc_json_file_engine::{JsonFileEngine, JsonFileRequirements};
use globset as _;

#[test]
fn engine_and_requirement_share_the_canonical_id() {
    use aqc_file_engine_core::{Engine as _, EngineRequirement as _};
    assert_eq!(JsonFileEngine.id(), "aqc-json-file-engine");
    assert_eq!(
        JsonFileRequirements::default().engine_id(),
        "aqc-json-file-engine"
    );
}
