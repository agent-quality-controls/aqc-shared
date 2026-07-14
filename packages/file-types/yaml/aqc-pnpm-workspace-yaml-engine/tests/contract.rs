use aqc_file_engine_core::{Engine, EngineRequirement};
use aqc_pnpm_workspace_yaml_engine::{
    ENGINE_ID, PnpmOnFail, PnpmReleaseAgeMinutes, PnpmTrustPolicy, PnpmWorkspaceYamlEngine,
    PnpmWorkspaceYamlRequirements,
};
use aqc_yaml_engine_core as _;
use globset as _;
use schemars as _;
use serde as _;

#[test]
fn engine_id_and_requirement_dispatch_match() {
    let engine = PnpmWorkspaceYamlEngine;
    let requirement = PnpmWorkspaceYamlRequirements::default();
    assert_eq!(engine.id(), ENGINE_ID);
    assert_eq!(requirement.engine_id(), ENGINE_ID);
}

#[test]
fn public_closed_values_have_expected_renderings() {
    assert_eq!(PnpmOnFail::Download.to_string(), "download");
    assert_eq!(PnpmOnFail::Error.to_string(), "error");
    assert_eq!(PnpmOnFail::Warn.to_string(), "warn");
    assert_eq!(PnpmOnFail::Ignore.to_string(), "ignore");
    assert_eq!(PnpmTrustPolicy::NoDowngrade.to_string(), "no-downgrade");
    assert_eq!(PnpmTrustPolicy::Off.to_string(), "off");
    assert_eq!(
        PnpmReleaseAgeMinutes::new(1440).map(|value| value.get()),
        Ok(1440)
    );
}
