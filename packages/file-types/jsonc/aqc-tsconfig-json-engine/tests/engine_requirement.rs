use aqc_file_engine_core::EngineRequirement;
use aqc_jsonc_engine_core as _;
use aqc_tsconfig_json_engine::{ENGINE_ID, TsconfigJsonRequirements};
use schemars as _;
use serde as _;

#[test]
fn requirement_reports_engine_id() {
    assert_eq!(TsconfigJsonRequirements::default().engine_id(), ENGINE_ID);
}
